use super::AttachmentId;
use crate::bridge::{EVENT_CAPACITY, Error, EventInterests, ScriptEvent};
use parking_lot::{Condvar, Mutex};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    task::Waker,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub id: u64,
    pub asked_by: u64,
    pub payload: serde_json::Value,
}

struct Route {
    asked_by: AttachmentId,
    deadline: Instant,
}
struct Mailbox {
    interests: EventInterests,
    items: VecDeque<ScriptEvent>,
    waker: Option<Waker>,
    failed: bool,
}
#[derive(Default)]
struct State {
    bot: Option<AttachmentId>,
    next: u64,
    links: HashMap<AttachmentId, Mailbox>,
    routes: HashMap<u64, Route>,
    stopped: bool,
}
#[derive(Default)]
pub(crate) struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}
pub(crate) struct Router {
    pub(crate) shared: Arc<Shared>,
    worker: Option<thread::JoinHandle<()>>,
}

fn wake(wakers: Vec<Waker>) {
    for waker in wakers {
        waker.wake();
    }
}

impl State {
    fn send(&mut self, to: AttachmentId, event: ScriptEvent, wakers: &mut Vec<Waker>) {
        let Some(mailbox) = self.links.get_mut(&to) else {
            return;
        };
        if !mailbox.interests.accepts(&event) {
            return;
        }
        if mailbox.items.len() >= EVENT_CAPACITY {
            mailbox.failed = true;
            mailbox.items.clear();
            self.routes.retain(|_, route| route.asked_by != to);
        } else if !mailbox.failed {
            mailbox.items.push_back(event);
        }
        if let Some(waker) = &mailbox.waker {
            wakers.push(waker.clone());
        }
    }
    fn expire(&mut self, now: Instant, wakers: &mut Vec<Waker>) {
        let ids: Vec<_> = self
            .routes
            .iter()
            .filter(|(_, route)| route.deadline <= now)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            let Some(route) = self.routes.remove(&id) else {
                continue;
            };
            self.send(
                route.asked_by,
                ScriptEvent::Answer {
                    id,
                    answer: Err(Error::BotRequestTimeout),
                },
                wakers,
            );
        }
    }
}

impl Router {
    pub(crate) fn start(session: u64) -> std::io::Result<Self> {
        let shared = Arc::new(Shared::default());
        let task = shared.clone();
        let worker = thread::Builder::new()
            .name(format!("asks-{session}"))
            .spawn(move || {
                let mut state = task.state.lock();
                while !state.stopped {
                    let mut wakers = Vec::new();
                    state.expire(Instant::now(), &mut wakers);
                    if !wakers.is_empty() {
                        drop(state);
                        wake(wakers);
                        state = task.state.lock();
                        continue;
                    }
                    if let Some(deadline) = state.routes.values().map(|route| route.deadline).min()
                    {
                        task.changed.wait_for(
                            &mut state,
                            deadline.saturating_duration_since(Instant::now()),
                        );
                    } else {
                        task.changed.wait(&mut state);
                    }
                }
            })?;
        Ok(Self {
            shared,
            worker: Some(worker),
        })
    }
}
impl Drop for Router {
    fn drop(&mut self) {
        let wakers = {
            let mut state = self.shared.state.lock();
            state.stopped = true;
            state.routes.clear();
            state
                .links
                .values()
                .filter_map(|mailbox| mailbox.waker.clone())
                .collect()
        };
        self.shared.changed.notify_all();
        wake(wakers);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl Shared {
    pub(crate) fn attach(&self, id: AttachmentId, interests: EventInterests) {
        self.state.lock().links.insert(
            id,
            Mailbox {
                interests,
                items: VecDeque::new(),
                waker: None,
                failed: false,
            },
        );
    }
    pub(crate) fn claim_bot(&self, id: AttachmentId) -> Result<(), Error> {
        let mut state = self.state.lock();
        if state.stopped || !state.links.contains_key(&id) {
            return Err(Error::Stopped);
        }
        if state.bot.is_some() {
            return Err(Error::BotAlreadyAttached);
        }
        state.bot = Some(id);
        Ok(())
    }
    pub(crate) fn ask(
        &self,
        asked_by: AttachmentId,
        payload: serde_json::Value,
    ) -> Result<u64, Error> {
        let mut wakers = Vec::new();
        let id = {
            let mut state = self.state.lock();
            if state.stopped
                || state
                    .links
                    .get(&asked_by)
                    .is_none_or(|mailbox| mailbox.failed)
            {
                return Err(Error::Stopped);
            }
            let bot = state.bot.ok_or(Error::BotUnavailable)?;
            if state.routes.len() >= EVENT_CAPACITY
                || state.links[&bot].items.len() >= EVENT_CAPACITY
                || state.links[&bot].failed
            {
                return Err(Error::BotQueueFull);
            }
            state.next = state.next.checked_add(1).expect("ask IDs exhausted");
            let id = state.next;
            state.routes.insert(
                id,
                Route {
                    asked_by,
                    deadline: Instant::now() + Duration::from_secs(5),
                },
            );
            state.send(
                bot,
                ScriptEvent::Ask {
                    request: Request {
                        id,
                        asked_by: asked_by.0,
                        payload,
                    },
                },
                &mut wakers,
            );
            id
        };
        self.changed.notify_one();
        wake(wakers);
        Ok(id)
    }
    pub(crate) fn answer(
        &self,
        bot: AttachmentId,
        id: u64,
        answer: Result<serde_json::Value, Error>,
    ) -> Result<(), Error> {
        let mut wakers = Vec::new();
        {
            let mut state = self.state.lock();
            if state.stopped {
                return Err(Error::Stopped);
            }
            if state.bot != Some(bot) {
                return Err(Error::BotAnswerDenied);
            }
            if let Some(route) = state.routes.remove(&id) {
                let answer = if Instant::now() >= route.deadline {
                    Err(Error::BotRequestTimeout)
                } else {
                    answer
                };
                state.send(
                    route.asked_by,
                    ScriptEvent::Answer { id, answer },
                    &mut wakers,
                );
            }
        }
        self.changed.notify_one();
        wake(wakers);
        Ok(())
    }
    pub(crate) fn is_pending(&self, bot: AttachmentId, id: u64) -> bool {
        let state = self.state.lock();
        !state.stopped
            && state.bot == Some(bot)
            && state
                .routes
                .get(&id)
                .is_some_and(|route| route.deadline > Instant::now())
    }
    pub(crate) fn cancel(&self, asker: AttachmentId, id: u64) {
        let mut state = self.state.lock();
        if state
            .routes
            .get(&id)
            .is_some_and(|route| route.asked_by == asker)
        {
            state.routes.remove(&id);
        }
        self.changed.notify_one();
    }
    pub(crate) fn detach(&self, id: AttachmentId) {
        let mut wakers = Vec::new();
        {
            let mut state = self.state.lock();
            state.links.remove(&id);
            state.routes.retain(|_, route| route.asked_by != id);
            if state.bot == Some(id) {
                state.bot = None;
                let routes = std::mem::take(&mut state.routes);
                for (id, route) in routes {
                    state.send(
                        route.asked_by,
                        ScriptEvent::Answer {
                            id,
                            answer: Err(Error::BotStopped),
                        },
                        &mut wakers,
                    );
                }
            }
        }
        self.changed.notify_one();
        wake(wakers);
    }
    pub(crate) fn try_event(&self, id: AttachmentId) -> Result<Option<ScriptEvent>, Error> {
        let mut state = self.state.lock();
        let stopped = state.stopped;
        let mailbox = state.links.get_mut(&id).ok_or(Error::Stopped)?;
        if mailbox.failed {
            return Err(Error::Stopped);
        }
        match mailbox.items.pop_front() {
            Some(item) => Ok(Some(item)),
            None if stopped => Err(Error::Stopped),
            None => Ok(None),
        }
    }
    pub(crate) fn set_waker(&self, id: AttachmentId, waker: Waker) {
        let pending = {
            let mut state = self.state.lock();
            let stopped = state.stopped;
            if let Some(mailbox) = state.links.get_mut(&id) {
                mailbox.waker = Some(waker.clone());
                stopped || mailbox.failed || !mailbox.items.is_empty()
            } else {
                true
            }
        };
        if pending {
            waker.wake();
        }
    }
    #[cfg(feature = "testing")]
    pub(crate) fn count(&self) -> usize {
        self.state.lock().routes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiring_a_full_answer_mailbox_drops_its_other_routes() {
        let asker = AttachmentId(1);
        let mut state = State::default();
        state.links.insert(
            asker,
            Mailbox {
                items: (0..EVENT_CAPACITY)
                    .map(|_| ScriptEvent::Invalidated)
                    .collect(),
                interests: EventInterests::ALL,
                waker: None,
                failed: false,
            },
        );
        for id in 0..3 {
            state.routes.insert(
                id,
                Route {
                    asked_by: asker,
                    deadline: Instant::now(),
                },
            );
        }
        state.expire(Instant::now(), &mut Vec::new());
        assert!(state.routes.is_empty());
        assert!(state.links[&asker].failed);
        assert!(state.links[&asker].items.is_empty());
    }
}
