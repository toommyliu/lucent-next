use super::*;
use std::sync::mpsc;

const REQUEST: &[u8] = b"<msg t='sys'><body action='verChk' r='0'><ver v='166' /></body></msg>";
const REPLY: &[u8] = b"<msg t='sys'><body action='apiOK' r='0'></body></msg>";
const TO_SERVER: &[u8] = include_bytes!("fixtures/handshake/to-server.bin");
const TO_CLIENT: &[u8] = include_bytes!("fixtures/handshake/to-client.bin");

#[test]
fn recorded_packets_survive_every_split_bytewise_and_coalesced_reads() {
    for (stream, expected) in [(TO_SERVER, REQUEST), (TO_CLIENT, REPLY)] {
        for split in 0..=stream.len() {
            let mut framer = Framer::default();
            let mut packets = Vec::new();
            framer
                .observe(&stream[..split], |_, p| packets.push(p.to_vec()))
                .unwrap();
            framer
                .observe(&stream[split..], |_, p| packets.push(p.to_vec()))
                .unwrap();
            assert_eq!(packets, [expected]);
            assert_eq!(framer.tail_len(), 0);
        }
        let mut framer = Framer::default();
        let mut packets = Vec::new();
        for byte in stream.chunks(1) {
            framer
                .observe(byte, |_, p| packets.push(p.to_vec()))
                .unwrap();
        }
        assert_eq!(packets, [expected]);
    }
    let mut framer = Framer::default();
    let mut packets = Vec::new();
    let stream = [TO_SERVER, b"\0", TO_CLIENT, b"partial"].concat();
    framer
        .observe(&stream, |n, p| packets.push((n, p.to_vec())))
        .unwrap();
    assert_eq!(
        packets,
        [(1, REQUEST.to_vec()), (2, vec![]), (3, REPLY.to_vec())]
    );
    assert_eq!(framer.tail_len(), 7);
    let mut fresh = Framer::default();
    fresh
        .observe(b"fresh\0", |n, p| {
            assert_eq!(n, 1);
            assert_eq!(p, b"fresh");
        })
        .unwrap();
}

#[test]
fn packet_limit_is_independent_of_chunking_and_terminator() {
    for terminated in [false, true] {
        for chunk in [1, 4096, MAX_PACKET_BYTES + 2] {
            for length in [MAX_PACKET_BYTES - 1, MAX_PACKET_BYTES, MAX_PACKET_BYTES + 1] {
                let mut stream = vec![b'x'; length];
                if terminated {
                    stream.push(0);
                }
                let mut framer = Framer::default();
                let mut result = Ok(());
                let mut lengths = Vec::new();
                for bytes in stream.chunks(chunk) {
                    result = framer.observe(bytes, |_, p| lengths.push(p.len()));
                    if result.is_err() {
                        break;
                    }
                }
                assert_eq!(result.is_err(), length > MAX_PACKET_BYTES);
                if terminated && length <= MAX_PACKET_BYTES {
                    assert_eq!(lengths, [length]);
                } else {
                    assert!(lengths.is_empty());
                }
            }
        }
    }
}

#[test]
fn socket_policy_uses_canonical_dns_label_boundaries() {
    for host in [
        "aq.com",
        "sock7.aq.com",
        "SOCK8.AQ.COM.",
        "euro.aqw.artix.com",
        "asia.game.artix.com",
    ] {
        assert!(GameHost::parse(host).is_some(), "{host}");
    }
    for host in [
        "127.0.0.1",
        "2130706433",
        "[::1]",
        "::1",
        "evil-aq.com",
        "aq.com.evil",
        "artix.com.evil",
        "aq.com:5588",
        "aq.com/path",
        "user@aq.com",
        "aq.com..",
        "",
    ] {
        assert!(GameHost::parse(host).is_none(), "{host}");
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Opened(ConnectionId),
    Packet(ConnectionId, Direction, u64, Vec<u8>),
    Closed(ConnectionId, CloseReason, [usize; 2]),
}

fn observer() -> (Observer, mpsc::Receiver<Seen>) {
    let (tx, rx) = mpsc::channel();
    let observer = Observer::new(move |event| {
        let seen = match event {
            Event::Opened { id, .. } => Seen::Opened(id),
            Event::Packet {
                id,
                direction,
                sequence,
                payload,
            } => Seen::Packet(id, direction, sequence, payload.to_vec()),
            Event::Closed { id, reason, tails } => Seen::Closed(id, reason, tails),
        };
        let _ = tx.send(seen);
    });
    (observer, rx)
}

fn next(rx: &mpsc::Receiver<Seen>) -> Seen {
    rx.recv_timeout(Duration::from_secs(5)).unwrap()
}

#[test]
fn real_socket_preserves_wire_bytes_observes_attempts_and_reports_partial_tails() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let mut stream = accept(&listener);
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = vec![0; TO_SERVER.len() + 4];
        stream.read_exact(&mut bytes).unwrap();
        assert_eq!(bytes, [TO_SERVER, b"tail"].concat());
        for chunk in TO_CLIENT.chunks(3) {
            stream.write_all(chunk).unwrap();
        }
        stream.write_all(b"left").unwrap();
    });
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (observer, seen) = observer();
    let net = Net::for_test(
        runtime.handle().clone(),
        observer,
        vec![("sock7.aq.com", addr)],
    );
    let (tx, rx) = async_channel::unbounded();
    let (events_tx, events_rx) = mpsc::channel();
    net.connect(
        "sock7.aq.com".into(),
        addr.port(),
        Duration::from_secs(5),
        rx,
        move |event| events_tx.send(event).is_ok(),
    );
    assert_eq!(
        events_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        SocketEvent::Connected
    );
    tx.send_blocking([TO_SERVER, b"tail"].concat()).unwrap();
    let mut received = Vec::new();
    loop {
        let event = events_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the socket never reported Closed");
        match event {
            SocketEvent::Data(bytes) => received.extend(bytes),
            SocketEvent::Closed => break,
            event => panic!("{event:?}"),
        }
    }
    assert_eq!(received, [TO_CLIENT, b"left"].concat());
    server.join().unwrap();
    let Seen::Opened(id) = next(&seen) else {
        panic!()
    };
    assert_eq!(
        next(&seen),
        Seen::Packet(id, Direction::ToServer, 1, REQUEST.to_vec())
    );
    assert_eq!(
        next(&seen),
        Seen::Packet(id, Direction::ToClient, 1, REPLY.to_vec())
    );
    assert_eq!(
        next(&seen),
        Seen::Closed(id, CloseReason::RemoteClosed, [4, 4])
    );
    drop(net);
    assert!(seen.recv_timeout(Duration::from_millis(50)).is_err());
}

#[test]
fn cancellation_closes_once_and_restart_retains_unique_connection_ids() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (observer, seen) = observer();
    let mut ids = Vec::new();
    for _ in 0..2 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let net = Net::for_test(
            runtime.handle().clone(),
            observer.clone(),
            vec![("sock7.aq.com", addr)],
        );
        let (_tx, rx) = async_channel::unbounded();
        let (connected, wait) = mpsc::channel();
        net.connect(
            "sock7.aq.com".into(),
            addr.port(),
            Duration::from_secs(5),
            rx,
            move |event| {
                if event == SocketEvent::Connected {
                    connected.send(()).unwrap();
                }
                true
            },
        );
        let _stream = accept(&listener);
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        let Seen::Opened(id) = next(&seen) else {
            panic!()
        };
        ids.push(id);
        drop(net);
        assert_eq!(
            next(&seen),
            Seen::Closed(id, CloseReason::Cancelled, [0, 0])
        );
    }
    assert_ne!(ids[0], ids[1]);
    assert!(seen.recv_timeout(Duration::from_millis(50)).is_err());
}

pub(super) fn accept(listener: &std::net::TcpListener) -> std::net::TcpStream {
    listener.set_nonblocking(true).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(error) => panic!("expected a socket connection: {error}"),
        }
    }
}

#[test]
fn forbidden_socket_hostnames_are_rejected_before_dialing() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (observer, seen) = observer();
    let net = Net::for_test(
        runtime.handle().clone(),
        observer,
        vec![("aq.com.evil", addr)],
    );
    let (_tx, rx) = async_channel::unbounded();
    let (events, received) = mpsc::channel();
    net.connect(
        "aq.com.evil".into(),
        addr.port(),
        Duration::from_secs(1),
        rx,
        move |event| events.send(event).is_ok(),
    );
    assert_eq!(
        received.recv_timeout(Duration::from_secs(1)).unwrap(),
        SocketEvent::Failed
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(seen.try_recv().is_err());
}

#[test]
fn host_close_precedes_the_frontend_close_notification() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        drop(accept(&listener));
    });
    let closed = Arc::new(AtomicBool::new(false));
    let host_closed = closed.clone();
    let observer = Observer::new(move |event| {
        if matches!(event, Event::Closed { .. }) {
            host_closed.store(true, Ordering::Release);
        }
    });
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let net = Net::for_test(
        runtime.handle().clone(),
        observer,
        vec![("sock7.aq.com", addr)],
    );
    let (_tx, rx) = async_channel::unbounded();
    let (finished, wait) = mpsc::channel();
    net.connect(
        "sock7.aq.com".into(),
        addr.port(),
        Duration::from_secs(5),
        rx,
        move |event| {
            if event == SocketEvent::Closed {
                finished.send(closed.load(Ordering::Acquire)).unwrap();
            }
            true
        },
    );
    assert!(wait.recv_timeout(Duration::from_secs(5)).unwrap());
    server.join().unwrap();
}
