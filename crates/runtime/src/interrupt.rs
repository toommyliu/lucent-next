use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::{Kill, RuntimeError};

const TURN_LIMIT: Duration = Duration::from_millis(200);
const RETERMINATE: Duration = Duration::from_millis(20);

pub(crate) struct Interrupt {
    state: Mutex<State>,
    alarm: Condvar,
}

struct State {
    isolate: Option<v8::IsolateHandle>,
    clock: Clock,
    stop: Option<Stop>,
    armed: Option<Instant>,
}

enum Clock {
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
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(State {
                isolate: None,
                clock: Clock::Idle,
                stop: None,
                armed: None,
            }),
            alarm: Condvar::new(),
        }
    }

    pub(crate) fn attach(&self, isolate: v8::IsolateHandle) {
        let mut state = self.state.lock().unwrap();
        if matches!(state.stop, Some(Stop::Shutdown)) {
            isolate.terminate_execution();
        }
        state.isolate = Some(isolate);
    }

    pub(crate) fn detach(&self) {
        let mut state = self.state.lock().unwrap();
        state.isolate = None;
        if matches!(state.stop, Some(Stop::Kill(_))) {
            state.stop = None;
        }
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
        self.alarm.notify_one();
    }

    pub(crate) fn begin_turn(&self) {
        let mut state = self.state.lock().unwrap();
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
        state.clock = Clock::Idle;
        match state.stop {
            Some(Stop::Kill(_)) => state.stop.take(),
            stop => stop,
        }
    }

    pub(crate) fn stop(&self) -> Option<Stop> {
        self.state.lock().unwrap().stop
    }

    fn rouse(&self, state: &State, deadline: Instant) {
        if state.armed.is_none_or(|armed| deadline < armed) {
            self.alarm.notify_one();
        }
    }

    pub(crate) fn watch(&self) {
        let mut state = self.state.lock().unwrap();
        loop {
            if matches!(state.stop, Some(Stop::Shutdown)) {
                return;
            }
            state = match state.clock {
                Clock::Running(deadline) => {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        state.kill(Kill::Watchdog);
                        state.clock = Clock::Running(Instant::now() + RETERMINATE);
                        continue;
                    }
                    state.armed = Some(deadline);
                    let mut state = self.alarm.wait_timeout(state, left).unwrap().0;
                    state.armed = None;
                    state
                }
                Clock::Idle | Clock::Blocked(_) => self.alarm.wait(state).unwrap(),
            };
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
