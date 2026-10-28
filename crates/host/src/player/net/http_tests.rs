use super::*;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

#[test]
fn deadline_releases_the_request_when_the_body_queue_is_full() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(error) => panic!("expected HTTP connection: {error}"),
            }
        };
        receive(&stream);
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 67108864\r\n\r\n")
            .unwrap();
        let chunk = [b'x'; 65536];
        for _ in 0..1024 {
            if let Err(error) = stream.write_all(&chunk) {
                return error.kind();
            }
        }
        panic!("the client consumed an unread 64 MiB response");
    });
    let (runtime, mut net) = runtime_net(addr);
    net.deadline = Duration::from_millis(250);
    let response = runtime
        .block_on(net.fetch(HttpRequest {
            method: Method::Get,
            url: url("game.aq.com", addr, "/large"),
            headers: vec![],
            body: None,
        }))
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(2), async {
            while !net
                .tasks
                .lock()
                .unwrap()
                .iter()
                .all(|task| task.is_finished())
            {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the HTTP task outlived its deadline while the consumer retained its body");
    });
    assert!(matches!(
        server.join().unwrap(),
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
    ));
    let result = runtime.block_on(async {
        while response.body.next_chunk().await?.is_some() {}
        Ok::<(), NetError>(())
    });
    assert!(matches!(result, Err(NetError::TimedOut)));
}

const EXPECTED_USER_AGENT: &str = if cfg!(target_os = "macos") {
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_16_0) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
} else if cfg!(target_os = "windows") {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
} else {
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36"
};

#[derive(Debug)]
struct Received {
    line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Received {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

fn receive(mut stream: &TcpStream) -> Received {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        head.push(byte[0]);
    }
    let head = String::from_utf8(head).unwrap();
    let mut lines = head.lines();
    let line = lines.next().unwrap().to_owned();
    let headers: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(n, v)| (n.to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let len = headers
        .iter()
        .find(|(n, _)| n == "content-length")
        .map(|(_, v)| v.parse().unwrap())
        .unwrap_or(0);
    let mut body = vec![0; len];
    stream.read_exact(&mut body).unwrap();
    Received {
        line,
        headers,
        body,
    }
}

fn serve(
    responses: impl FnOnce(SocketAddr) -> Vec<(Duration, String)>,
) -> (SocketAddr, std::thread::JoinHandle<Vec<Received>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let responses = responses(addr);
    listener.set_nonblocking(true).unwrap();
    let thread = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for (delay, response) in responses {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("expected an HTTP connection: {e}"),
                }
            };
            requests.push(receive(&stream));
            std::thread::sleep(delay);
            let _ = stream.write_all(response.as_bytes());
        }
        requests
    });
    (addr, thread)
}

fn answer(status: &str, headers: &str, body: &str) -> (Duration, String) {
    (
        Duration::ZERO,
        format!(
            "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}",
            body.len()
        ),
    )
}

fn url(host: &str, addr: SocketAddr, path: &str) -> Url {
    Url::parse(&format!("http://{host}:{}{path}", addr.port())).unwrap()
}

fn load(
    runtime: &tokio::runtime::Runtime,
    net: &Net,
    method: Method,
    url: Url,
    headers: Vec<(String, String)>,
    body: Option<(Vec<u8>, String)>,
) -> Result<(Url, u16, Option<String>, Vec<u8>), NetError> {
    runtime.block_on(async {
        let response = net
            .fetch(HttpRequest {
                method,
                url,
                headers,
                body,
            })
            .await?;
        let mut body = Vec::new();
        while let Some(chunk) = response.body.next_chunk().await? {
            body.extend(chunk);
        }
        Ok((response.url, response.status, response.content_type, body))
    })
}

fn runtime_net(addr: SocketAddr) -> (tokio::runtime::Runtime, Net) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let net = Net::for_test(
        runtime.handle().clone(),
        Observer::default(),
        vec![
            ("game.aq.com", addr),
            ("other.artix.com", addr),
            ("outside.example", addr),
        ],
    );
    (runtime, net)
}

fn overrides() -> Vec<(String, String)> {
    [
        ("UsEr-AgEnT", "spoofed"),
        ("ARTIXMODE", "spoofed"),
        ("Cookie", "caller=secret"),
        ("Authorization", "Bearer secret"),
        ("Proxy-Authorization", "secret"),
        ("Host", "spoof.example"),
        ("Origin", "http://game.aq.com"),
        ("Referer", "http://game.aq.com/private"),
    ]
    .into_iter()
    .map(|(n, v)| (n.into(), v.into()))
    .collect()
}

#[test]
fn non_game_requests_do_not_carry_launcher_identity() {
    let (addr, server) = serve(|_| vec![answer("200 OK", "", "ok")]);
    let (runtime, net) = runtime_net(addr);
    assert_eq!(
        load(
            &runtime,
            &net,
            Method::Get,
            url("outside.example", addr, "/"),
            overrides(),
            None
        )
        .unwrap()
        .3,
        b"ok"
    );
    let requests = server.join().unwrap();
    assert_eq!(requests[0].header("user-agent"), None);
    assert_eq!(requests[0].header("artixmode"), None);
    assert_eq!(requests[0].header("cookie"), None);
}

#[test]
fn redirects_recompute_identity_in_both_directions_and_scope_cookies() {
    let (addr, server) = serve(|addr| {
        vec![
            answer(
                "302 Found",
                &format!(
                    "Set-Cookie: game=private; Path=/\r\nLocation: {}\r\n",
                    url("outside.example", addr, "/away")
                ),
                "",
            ),
            answer(
                "302 Found",
                &format!(
                    "Set-Cookie: foreign=ignored; Domain=aq.com; Path=/\r\nLocation: {}\r\n",
                    url("game.aq.com", addr, "/back")
                ),
                "",
            ),
            answer(
                "200 OK",
                "Content-Type: text/plain; charset=utf-8\r\n",
                "body",
            ),
        ]
    });
    let (runtime, net) = runtime_net(addr);
    let result = load(
        &runtime,
        &net,
        Method::Get,
        url("game.aq.com", addr, "/start"),
        overrides(),
        None,
    )
    .unwrap();
    assert_eq!(
        result,
        (
            url("game.aq.com", addr, "/back"),
            200,
            Some("text/plain; charset=utf-8".into()),
            b"body".to_vec()
        )
    );
    let requests = server.join().unwrap();
    assert_eq!(requests[0].header("user-agent"), Some(EXPECTED_USER_AGENT));
    assert_eq!(requests[0].header("artixmode"), Some("launcher"));
    assert_eq!(requests[0].header("cookie"), Some("caller=secret"));
    assert_eq!(requests[1].header("user-agent"), None);
    assert_eq!(requests[1].header("artixmode"), None);
    assert_eq!(requests[1].header("cookie"), None);
    for name in ["authorization", "proxy-authorization", "origin", "referer"] {
        assert_eq!(requests[1].header(name), None, "{name}");
    }
    assert_eq!(
        requests[1].header("host"),
        Some(format!("outside.example:{}", addr.port()).as_str())
    );
    assert_eq!(requests[2].header("user-agent"), Some(EXPECTED_USER_AGENT));
    assert_eq!(requests[2].header("artixmode"), Some("launcher"));
    assert_eq!(requests[2].header("cookie"), Some("game=private"));
}

#[test]
fn cross_game_redirects_retain_launcher_identity_without_origin_credentials() {
    let (addr, server) = serve(|addr| {
        vec![
            answer(
                "307 Temporary Redirect",
                &format!("Location: {}\r\n", url("other.artix.com", addr, "/final")),
                "",
            ),
            answer("200 OK", "", ""),
        ]
    });
    let (runtime, net) = runtime_net(addr);
    load(
        &runtime,
        &net,
        Method::Get,
        url("game.aq.com", addr, "/start"),
        overrides(),
        None,
    )
    .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests[1].header("user-agent"), Some(EXPECTED_USER_AGENT));
    assert_eq!(requests[1].header("artixmode"), Some("launcher"));
    assert_eq!(requests[1].header("authorization"), None);
    assert_eq!(requests[1].header("cookie"), None);
}

#[test]
fn post_redirects_change_or_preserve_the_method_body_and_headers() {
    for status in [301, 302, 303, 307, 308] {
        let (addr, server) = serve(|_| {
            vec![
                answer(&format!("{status} Redirect"), "Location: /final\r\n", ""),
                answer("200 OK", "", ""),
            ]
        });
        let (runtime, net) = runtime_net(addr);
        load(
            &runtime,
            &net,
            Method::Post,
            url("game.aq.com", addr, "/start"),
            vec![
                ("Content-Encoding".into(), "identity".into()),
                ("Content-Language".into(), "en".into()),
                ("Content-Location".into(), "/body".into()),
            ],
            Some((b"message".to_vec(), "application/test".into())),
        )
        .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests[0].line, "POST /start HTTP/1.1");
        assert_eq!(requests[0].body, b"message");
        if status == 307 || status == 308 {
            assert_eq!(requests[1].line, "POST /final HTTP/1.1");
            assert_eq!(requests[1].body, b"message");
            assert_eq!(requests[1].header("content-type"), Some("application/test"));
            assert_eq!(requests[1].header("content-encoding"), Some("identity"));
            assert_eq!(requests[1].header("content-language"), Some("en"));
            assert_eq!(requests[1].header("content-location"), Some("/body"));
        } else {
            assert_eq!(requests[1].line, "GET /final HTTP/1.1");
            assert_eq!(requests[1].body, b"");
            for name in [
                "content-type",
                "content-length",
                "content-encoding",
                "content-language",
                "content-location",
                "transfer-encoding",
            ] {
                assert_eq!(requests[1].header(name), None);
            }
        }
    }
}

#[test]
fn redirect_and_body_share_one_deadline() {
    let (addr, server) = serve(|_| {
        vec![
            (
                Duration::from_millis(160),
                answer("302 Found", "Location: /final\r\n", "").1,
            ),
            (Duration::from_millis(160), answer("200 OK", "", "body").1),
        ]
    });
    let (runtime, mut net) = runtime_net(addr);
    net.deadline = Duration::from_millis(260);
    assert!(matches!(
        load(
            &runtime,
            &net,
            Method::Get,
            url("game.aq.com", addr, "/start"),
            vec![],
            None
        ),
        Err(NetError::TimedOut)
    ));
    server.join().unwrap();
}

#[test]
fn redirects_are_bounded_and_reject_non_http_targets() {
    let (addr, server) = serve(|_| {
        vec![answer(
            "302 Found",
            "Location: file:///private/test\r\n",
            "",
        )]
    });
    let (runtime, net) = runtime_net(addr);
    assert!(
        matches!(load(&runtime, &net, Method::Get, url("game.aq.com", addr, "/"), vec![], None), Err(NetError::Http(message)) if message.contains("only HTTP"))
    );
    server.join().unwrap();
    assert!(
        matches!(load(&runtime, &net, Method::Get, Url::parse("file:///private/test").unwrap(), vec![], None), Err(NetError::Http(message)) if message.contains("only HTTP"))
    );
    let (addr, server) = serve(|_| {
        (0..=10)
            .map(|_| answer("302 Found", "Location: /loop\r\n", ""))
            .collect()
    });
    let (runtime, net) = runtime_net(addr);
    assert!(
        matches!(load(&runtime, &net, Method::Get, url("game.aq.com", addr, "/loop"), vec![], None), Err(NetError::Http(message)) if message.contains("too many"))
    );
    assert_eq!(server.join().unwrap().len(), 11);
}

#[test]
fn non_game_cookies_are_neither_stored_nor_replayed() {
    let (addr, server) = serve(|_| {
        vec![
            answer("200 OK", "Set-Cookie: external=private; Path=/\r\n", ""),
            answer("200 OK", "", ""),
        ]
    });
    let (runtime, net) = runtime_net(addr);
    for _ in 0..2 {
        load(
            &runtime,
            &net,
            Method::Get,
            url("outside.example", addr, "/"),
            vec![],
            None,
        )
        .unwrap();
    }
    let requests = server.join().unwrap();
    assert_eq!(requests[1].header("cookie"), None);
}

#[test]
fn same_origin_redirect_retains_game_cookies_and_authorization() {
    let (addr, server) = serve(|_| {
        vec![
            answer(
                "302 Found",
                "Set-Cookie: game=private; Path=/\r\nLocation: /final\r\n",
                "",
            ),
            answer("200 OK", "", ""),
        ]
    });
    let (runtime, net) = runtime_net(addr);
    load(
        &runtime,
        &net,
        Method::Get,
        url("game.aq.com", addr, "/"),
        vec![("Authorization".into(), "Bearer retained".into())],
        None,
    )
    .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests[1].header("cookie"), Some("game=private"));
    assert_eq!(requests[1].header("authorization"), Some("Bearer retained"));
}

#[test]
fn time_spent_redirecting_counts_against_the_final_body_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let mut first = super::socket_tests::accept(&listener);
        receive(&first);
        std::thread::sleep(Duration::from_millis(160));
        first
            .write_all(answer("302 Found", "Location: /final\r\n", "").1.as_bytes())
            .unwrap();
        drop(first);
        let mut second = super::socket_tests::accept(&listener);
        receive(&second);
        second
            .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 4\r\n\r\n")
            .unwrap();
        std::thread::sleep(Duration::from_millis(160));
        let _ = second.write_all(b"body");
    });
    let (runtime, mut net) = runtime_net(addr);
    net.deadline = Duration::from_millis(260);
    assert!(matches!(
        load(
            &runtime,
            &net,
            Method::Get,
            url("game.aq.com", addr, "/"),
            vec![],
            None
        ),
        Err(NetError::TimedOut)
    ));
    server.join().unwrap();
}
