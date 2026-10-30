#[path = "../../runtime/tests/common/logs.rs"]
mod logs;

use lucent_host::{
    bridge::{Error, Value},
    testing::{self, ProbeMovie},
};
use lucent_protocol::{command::Command, settings::SettingsPatch};
use std::time::Duration;
use tracing_subscriber::prelude::*;

const WAIT: Duration = Duration::from_secs(5);

#[test]
fn rejected_settings_warn_once_until_the_game_accepts_them() {
    let logs = logs::Logs::default();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(logs.clone()))
        .unwrap();
    let movie = [
        "settings.setAnimationsEnabled",
        "settings.setCollisionsEnabled",
        "settings.setDeathAdsVisible",
        "settings.setOtherPlayersVisible",
        "settings.setLagKillerEnabled",
    ]
    .into_iter()
    .fold(ProbeMovie::new(), |movie, name| {
        movie.constant(name, serde_json::Value::Null)
    })
    .recording_when_ready("settings.setWalkSpeed")
    .throwing("settings.setFrameRate", "frame boom");
    let host = testing::headless_host();
    let (_session, link) = testing::start_probe(&host, movie);
    let walk_error = Error::LoaderThrew {
        callback: "settings.setWalkSpeed".into(),
        message: "settings called before readiness".into(),
    };
    for _ in 0..3 {
        assert_eq!(
            link.command(Command::UpdateSettings {
                patch: SettingsPatch {
                    walk_speed: Some(30.0),
                    frame_rate: Some(24.0),
                    ..Default::default()
                }
            })
            .recv_timeout(WAIT),
            Err(walk_error.clone())
        );
        assert_eq!(
            link.command(Command::ReapplySettings {
                actions_only: false
            })
            .recv_timeout(WAIT),
            Err(walk_error.clone())
        );
    }
    assert_eq!(
        logs.messages(tracing::Level::WARN),
        vec!["couldn't apply game setting"; 2]
    );
    link.call_loader("probe.ready", vec![])
        .recv_timeout(WAIT)
        .unwrap();
    for _ in 0..2 {
        assert_eq!(
            link.command(Command::ReapplySettings {
                actions_only: false
            })
            .recv_timeout(WAIT),
            Err(Error::LoaderThrew {
                callback: "settings.setFrameRate".into(),
                message: "frame boom".into(),
            })
        );
    }
    assert_eq!(
        logs.messages(tracing::Level::WARN),
        vec!["couldn't apply game setting"; 2]
    );
    assert_eq!(
        logs.messages(tracing::Level::INFO)
            .into_iter()
            .filter(|message| message == "game setting accepted again")
            .collect::<Vec<_>>(),
        vec!["game setting accepted again"]
    );
    assert_eq!(
        link.call_loader("probe.echo", vec![Value::Number(1.0)])
            .recv_timeout(WAIT),
        Ok(Value::Number(1.0))
    );
}
