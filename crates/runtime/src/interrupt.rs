use std::io;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::alarm::Alarm;
use crate::{Kill, RuntimeError};

const TURN_LIMIT: Duration = Duration::from_millis(200);
const RETERMINATE: Duration = Duration::from_millis(20);

pub(crate) struct Interrupt {
    state: Mutex<State>,
    alarm: Alarm,
}

struct State {
    isolate: Option<v8::IsolateHandle>,
    clock: Clock,
    stop: Option<Stop>,
    armed: Option<Instant>,
}

enum Clock {
    StartingScript,
    Idle,
    Running(Instant),
    Blocked(Duration),
}

#[derive(Clone, Copy)]
pub(crate) enum Stop {
    Kill(Kill),
    Shutdown,
}

impl Interrupt {
    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self {
            state: Mutex::new(State {
                isolate: None,
                clock: Clock::Idle,
                stop: None,
                armed: None,
            }),
            alarm: Alarm::new()?,
        })
    }

    pub(crate) fn attach(&self, isolate: v8::IsolateHandle) {
        let mut state = self.state.lock().unwrap();
        if state.stop.is_some() {
            isolate.terminate_execution();
        }
        state.isolate = Some(isolate);
    }

    pub(crate) fn detach(&self) {
        let mut state = self.state.lock().unwrap();
        state.isolate = None;
    }

    pub(crate) fn kill(&self, kill: Kill) {
        self.state.lock().unwrap().kill(kill);
    }

    pub(crate) fn shut_down(&self) {
        let mut state = self.state.lock().unwrap();
        state.stop = Some(Stop::Shutdown);
        if let Some(isolate) = &state.isolate {
            isolate.terminate_execution();
        }
        self.alarm.ring();
    }

    pub(crate) fn prepare_script(&self) {
        self.state.lock().unwrap().clock = Clock::StartingScript;
    }

    pub(crate) fn start_script(&self) {
        let mut state = self.state.lock().unwrap();
        if matches!(state.clock, Clock::StartingScript) {
            let deadline = Instant::now() + TURN_LIMIT;
            state.clock = Clock::Running(deadline);
            self.rouse(&state, deadline);
        }
    }

    pub(crate) fn begin_turn(&self) {
        let mut state = self.state.lock().unwrap();
        if matches!(state.clock, Clock::StartingScript) {
            return;
        }
        let deadline = Instant::now() + TURN_LIMIT;
        state.clock = Clock::Running(deadline);
        self.rouse(&state, deadline);
    }

    pub(crate) fn block(&self) {
        let mut state = self.state.lock().unwrap();
        if let Clock::Running(deadline) = state.clock {
            state.clock = Clock::Blocked(deadline.saturating_duration_since(Instant::now()));
        }
    }

    pub(crate) fn unblock(&self) {
        let mut state = self.state.lock().unwrap();
        if let Clock::Blocked(left) = state.clock {
            let deadline = Instant::now() + left;
            state.clock = Clock::Running(deadline);
            self.rouse(&state, deadline);
        }
    }

    pub(crate) fn end_turn(&self) -> Option<Stop> {
        let mut state = self.state.lock().unwrap();
        if !matches!(state.clock, Clock::StartingScript) {
            state.clock = Clock::Idle;
        }
        state.stop
    }

    pub(crate) fn stop(&self) -> Option<Stop> {
        self.state.lock().unwrap().stop
    }

    fn rouse(&self, state: &State, deadline: Instant) {
        if state.armed.is_none_or(|armed| deadline < armed) {
            self.alarm.ring();
        }
    }

    pub(crate) fn watch(&self) {
        let mut state = self.state.lock().unwrap();
        loop {
            if matches!(state.stop, Some(Stop::Shutdown)) {
                return;
            }
            let until = match state.clock {
                Clock::Running(deadline) if deadline <= Instant::now() => {
                    state.kill(Kill::Watchdog);
                    state.clock = Clock::Running(Instant::now() + RETERMINATE);
                    continue;
                }
                Clock::Running(deadline) => Some(deadline),
                Clock::StartingScript | Clock::Idle | Clock::Blocked(_) => None,
            };
            state.armed = until;
            state = self.alarm.wait(&self.state, state, until);
            state.armed = None;
        }
    }
}

impl State {
    fn kill(&mut self, kill: Kill) {
        let Some(isolate) = &self.isolate else {
            return;
        };
        if self.stop.is_none() {
            self.stop = Some(Stop::Kill(kill));
        }
        isolate.terminate_execution();
    }
}

impl From<Stop> for RuntimeError {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::Kill(kill) => RuntimeError::Killed(kill),
            Stop::Shutdown => RuntimeError::Stopped,
        }
    }
}
