use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::Duration;

use lucent_host::bridge::{Error, Value};
use lucent_host::testing::{self, ProbeMovie};
use serde_json::json;

#[test]
fn reads_before_the_viewport_are_empty_and_stop_with_the_session() {
    let host = testing::headless_host();
    let mut session = host.start_session(testing::headless_surface()).unwrap();
    let link = session.take_script_link().unwrap();
    assert_eq!(
        link.read(|projection| (
            projection.world.map.name.clone(),
            projection.world.get_me().map(|me| me.data().entity_id)
        )),
        Ok((String::new(), None))
    );
    drop(session);
    assert_eq!(link.read(|_| 42), Err(Error::Stopped));
}

#[test]
fn concurrent_reads_observe_the_area_and_hydrated_self_together() {
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, ProbeMovie::new());
    let link = Mutex::new(link);
    let area = json!({
        "type": "json",
        "dataObj": {
            "cmd": "moveToArea", "areaName": "testroom-1",
            "uoBranch": [{"entID": 7, "uoName": "player-a"}]
        }
    })
    .to_string();
    let (done, finished) = mpsc::channel::<()>();
    let (ready, reading) = mpsc::channel();
    let (half, post, total) = thread::scope(|scope| {
        let reader_link = &link;
        let reader = scope.spawn(move || {
            let (mut half, mut post, mut total) = (0_u64, 0_u64, 0_u64);
            ready.send(()).unwrap();
            loop {
                let (in_area, me) = reader_link
                    .lock()
                    .unwrap()
                    .read(|projection| {
                        (
                            projection.world.map.name == "testroom",
                            projection.world.get_me().map(|me| me.data().entity_id),
                        )
                    })
                    .unwrap();
                total += 1;
                if in_area && me.is_none() {
                    half += 1;
                }
                if in_area && me == Some(7) {
                    post += 1;
                }
                if !matches!(finished.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                    break;
                }
            }
            (half, post, total)
        });
        reading.recv_timeout(Duration::from_secs(5)).unwrap();
        for _ in 0..2_000 {
            for (name, payload) in [
                ("onConnection", "OnConnection"),
                ("onExtensionResponse", area.as_str()),
            ] {
                let pending = link.lock().unwrap().call_loader(
                    "probe.emit",
                    vec![Value::String(name.into()), Value::String(payload.into())],
                );
                assert_eq!(
                    pending.recv_timeout(Duration::from_secs(5)),
                    Ok(Value::Bool(true))
                );
            }
        }
        drop(done);
        reader.join().unwrap()
    });
    println!("half={half} post={post} total={total}");
    assert_eq!(half, 0);
    assert!(post > 0);
}
