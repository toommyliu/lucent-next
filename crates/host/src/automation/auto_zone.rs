use std::{
    collections::hash_map::RandomState,
    hash::BuildHasher,
    task::Waker,
    time::{Duration, Instant},
};

use lucent_protocol::{
    auto_zone::{AutoZoneConfig, AutoZoneMap},
    event::ProjectionEvent,
    query::Query,
};

use crate::{
    ScriptLink,
    bridge::{Error, Pending, ScriptEvent, Value},
};

const RETRY: Duration = Duration::from_millis(500);
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

pub struct AutoZonePolicy {
    link: ScriptLink,
    state: AutoZone,
    waker: Waker,
}

impl AutoZonePolicy {
    pub fn new(link: ScriptLink, waker: Waker) -> Self {
        link.set_waker(waker.clone());
        Self {
            link,
            state: AutoZone::new(Instant::now()),
            waker,
        }
    }

    pub fn advance(&mut self, now: Instant) -> Option<Instant> {
        for _ in 0..64 {
            match self.link.try_event() {
                Ok(Some(ScriptEvent::AutoZoneChanged { config })) => self.state.configure(config),
                Ok(Some(ScriptEvent::Loader { projected, .. })) => {
                    for event in projected {
                        if let ProjectionEvent::Zone { map, zone } = event {
                            self.state.zone(map, zone, now);
                        }
                    }
                }
                Ok(Some(ScriptEvent::Invalidated)) | Err(Error::Overflow) => {
                    self.state.refresh(now)
                }
                Ok(Some(_)) => {}
                Ok(None) => return self.state.advance(&self.link, &self.waker, now),
                Err(_) => return None,
            }
        }
        Some(now)
    }
}

#[derive(Clone, Copy)]
enum Charge {
    Positive,
    Negative,
    None,
}

#[derive(Clone)]
struct Snapshot {
    map: String,
    alive: bool,
    charge: Charge,
}

trait AutoZoneHost {
    fn config(&self) -> Result<String, String>;
    fn snapshot(&self) -> Result<Snapshot, String>;
    fn walk(&self, x: i32, y: i32) -> Pending<Value>;
}

impl AutoZoneHost for ScriptLink {
    fn config(&self) -> Result<String, String> {
        self.query_for(Duration::ZERO, &mut Some(Query::AutoZone {}))
            .map_err(|error| error.to_string())
    }
    fn snapshot(&self) -> Result<Snapshot, String> {
        self.read_for(Duration::ZERO, |projection| {
            let player = projection.world.get_me();
            let has = |names: [&str; 2]| {
                player.as_ref().is_some_and(|player| {
                    names.iter().any(|name| player.aura(name, None).is_some())
                })
            };
            Snapshot {
                map: projection.world.map.name.clone(),
                alive: player
                    .as_ref()
                    .is_some_and(|player| player.data().entity.alive()),
                charge: if has(["Positive Charge", "Positive Charge?"]) {
                    Charge::Positive
                } else if has(["Negative Charge", "Negative Charge?"]) {
                    Charge::Negative
                } else {
                    Charge::None
                },
            }
        })
        .map_err(|error| error.to_string())
    }
    fn walk(&self, x: i32, y: i32) -> Pending<Value> {
        self.call_loader(
            "player.walkTo",
            vec![Value::Number(f64::from(x)), Value::Number(f64::from(y))],
        )
    }
}

struct AutoZone {
    config: Option<AutoZoneConfig>,
    refresh_at: Option<Instant>,
    transition: Option<Transition>,
    failing: bool,
}

struct Transition {
    map: AutoZoneMap,
    zone: String,
    state: State,
}

enum State {
    Scheduled(Instant),
    Walking(Pending<Value>, Instant),
}
impl State {
    fn deadline(&self) -> Instant {
        match self {
            Self::Scheduled(at) | Self::Walking(_, at) => *at,
        }
    }
}

impl AutoZone {
    fn new(now: Instant) -> Self {
        Self {
            config: None,
            refresh_at: Some(now),
            transition: None,
            failing: false,
        }
    }

    fn configure(&mut self, config: AutoZoneConfig) {
        if !config.enabled
            || self
                .transition
                .as_ref()
                .is_some_and(|transition| Some(transition.map) != config.map)
        {
            self.transition = None;
        }
        self.config = Some(config);
    }

    fn zone(&mut self, map: String, zone: String, now: Instant) {
        let Ok(map) = serde_json::from_value::<AutoZoneMap>(serde_json::Value::String(map)) else {
            return;
        };
        if self
            .config
            .as_ref()
            .is_some_and(|config| !config.enabled || config.map != Some(map))
        {
            return;
        }
        self.transition = Some(Transition {
            map,
            zone,
            state: State::Scheduled(now + delay(map)),
        });
    }

    fn refresh(&mut self, now: Instant) {
        self.config = None;
        self.transition = None;
        self.refresh_at = Some(now);
    }

    fn advance(
        &mut self,
        host: &impl AutoZoneHost,
        waker: &Waker,
        now: Instant,
    ) -> Option<Instant> {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.step(host, waker, now)))
                .unwrap_or_else(|panic| {
                    let message = panic
                        .downcast_ref::<&str>()
                        .map(|text| (*text).to_owned())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_default();
                    Err(format!("auto zone transition panicked: {message}"))
                });
        if let Err(error) = result {
            if !self.failing {
                tracing::warn!("auto zone tick failed: {error}");
            }
            self.failing = true;
            if self.refresh_at.is_some() {
                self.refresh_at = Some(now + RETRY);
            } else if let Some(transition) = &mut self.transition {
                transition.state = State::Scheduled(now + RETRY + delay(transition.map));
            }
        }
        self.refresh_at.or_else(|| {
            self.transition
                .as_ref()
                .map(|transition| transition.state.deadline())
        })
    }

    fn recovered(&mut self) {
        if std::mem::take(&mut self.failing) {
            tracing::info!("auto zone recovered");
        }
    }

    fn step(
        &mut self,
        host: &impl AutoZoneHost,
        waker: &Waker,
        now: Instant,
    ) -> Result<(), String> {
        if let Some(at) = self.refresh_at {
            if now < at {
                return Ok(());
            }
            let raw = host.config()?;
            let value: serde_json::Value =
                serde_json::from_str(&raw).map_err(|error| error.to_string())?;
            let enabled = value
                .get("enabled")
                .and_then(serde_json::Value::as_bool)
                .ok_or("Invalid auto zone enabled value")?;
            let map =
                serde_json::from_value(value.get("map").ok_or("Missing auto zone map")?.clone())
                    .map_err(|error| error.to_string())?;
            self.configure(AutoZoneConfig { enabled, map });
            self.refresh_at = None;
            self.recovered();
        }
        let Some(transition) = &mut self.transition else {
            return Ok(());
        };
        if let State::Scheduled(at) = transition.state {
            if now < at {
                return Ok(());
            }
            let snapshot = host.snapshot()?;
            if self
                .config
                .as_ref()
                .is_none_or(|config| !config.enabled || config.map != Some(transition.map))
                || !snapshot.alive
                || !snapshot.map.eq_ignore_ascii_case(map_name(transition.map))
            {
                self.transition = None;
                self.recovered();
                return Ok(());
            }
            let Some([x, y]) = target(transition.map, &transition.zone, snapshot.charge) else {
                self.transition = None;
                self.recovered();
                return Ok(());
            };
            let random = RandomState::new().hash_one(now);
            let x = x[0] + (random % (x[1] - x[0] + 1) as u64) as i32;
            let y = y[0] + ((random >> 32) % (y[1] - y[0] + 1) as u64) as i32;
            let reply = host.walk(x, y);
            reply.set_waker(waker.clone());
            transition.state = State::Walking(reply, now + REPLY_TIMEOUT);
        }
        if let State::Walking(reply, at) = &transition.state
            && (reply.try_recv().is_some() || now >= *at)
        {
            self.transition = None;
            self.recovered();
        }
        Ok(())
    }
}

fn delay(map: AutoZoneMap) -> Duration {
    if map == AutoZoneMap::Queeniona {
        Duration::from_millis(500)
    } else {
        Duration::ZERO
    }
}

fn map_name(map: AutoZoneMap) -> &'static str {
    match map {
        AutoZoneMap::Ledgermayne => "ledgermayne",
        AutoZoneMap::Moreskulls => "moreskulls",
        AutoZoneMap::Ultradage => "ultradage",
        AutoZoneMap::Darkcarnax => "darkcarnax",
        AutoZoneMap::Astralshrine => "astralshrine",
        AutoZoneMap::Queeniona => "queeniona",
        AutoZoneMap::Magnumopus => "magnumopus",
    }
}

fn target(map: AutoZoneMap, zone: &str, charge: Charge) -> Option<[[i32; 2]; 2]> {
    let zones = match map {
        AutoZoneMap::Ledgermayne => [
            [[431, 547], [234, 239]],
            [[147, 276], [353, 357]],
            [[727, 852], [353, 356]],
        ],
        AutoZoneMap::Moreskulls => [
            [[778, 806], [358, 361]],
            [[696, 802], [445, 452]],
            [[677, 766], [321, 324]],
        ],
        AutoZoneMap::Ultradage => [
            [[481, 483], [296, 300]],
            [[49, 164], [406, 412]],
            [[797, 900], [400, 402]],
        ],
        AutoZoneMap::Darkcarnax => [
            [[480, 530], [419, 432]],
            [[731, 850], [431, 432]],
            [[54, 155], [431, 432]],
        ],
        AutoZoneMap::Astralshrine => [
            [[461, 465], [320, 325]],
            [[643, 708], [445, 447]],
            [[199, 287], [181, 205]],
        ],
        AutoZoneMap::Magnumopus => [
            [[466, 470], [344, 420]],
            [[682, 813], [367, 384]],
            [[170, 285], [377, 384]],
        ],
        AutoZoneMap::Queeniona => {
            return match (zone, charge) {
                ("A", Charge::Positive) | ("B", Charge::Negative) => Some([[746, 869], [369, 379]]),
                ("A", Charge::Negative) | ("B", Charge::Positive) => Some([[111, 272], [369, 379]]),
                ("A" | "B", Charge::None) => None,
                _ => Some([[490, 490], [320, 320]]),
            };
        }
    };
    match zone {
        "" => Some(zones[0]),
        "A" => Some(zones[1]),
        "B" => Some(zones[2]),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
