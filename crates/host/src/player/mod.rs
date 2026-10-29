mod backends;
mod commands;
pub(crate) mod frames;
mod net;
mod projection;
pub(crate) mod surface;
pub(crate) mod view;

use lucent_protocol::settings::GameSettings;
use parking_lot::RwLock;
use std::ops::ControlFlow;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use ruffle_core::{FloatDuration, Player, PlayerBuilder, config::Letterbox, tag_utils::SwfMovie};

pub(crate) use backends::{Audio, Clipboard, audio};
pub(crate) use surface::Surface;
pub(crate) use view::{ViewEvent, Viewport};

use crate::bridge::{
    Bridge, Call, Error as BridgeError, Events, PendingCall, PendingRead, ScriptEventSender, Value,
};
use crate::projection::Projection;
use backends::{Navigator, Tasks, Ui};
use net::Net;
use view::Effect;

const STOP_TIMEOUT: Duration = Duration::from_secs(2);
const MIN_INBOX_DRAIN: Duration = Duration::from_millis(2);
const LOADER_SWF: &[u8] = include_bytes!("../../../../assets/loader.swf");

#[derive(Clone)]
pub(crate) struct Movie(pub(crate) Arc<[u8]>);

impl Movie {
    pub(crate) fn loader() -> Self {
        static LOADER: LazyLock<Movie> = LazyLock::new(|| Movie(Arc::from(LOADER_SWF)));
        LOADER.clone()
    }
}

pub(crate) struct Options {
    pub(crate) movie: Movie,
    pub(crate) clipboard: Clipboard,
    pub(crate) audio: Audio,
    pub(crate) muted: bool,
    pub(crate) settings: Arc<RwLock<GameSettings>>,
}

pub(crate) struct Handle {
    pub(crate) inbox: Inbox,
    pub(crate) projection: Arc<RwLock<Projection>>,
    pub(crate) stopped: Arc<AtomicBool>,
    exited: mpsc::Receiver<Result<(), PlayerError>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Handle {
    pub(crate) fn send(&self, event: ViewEvent) {
        self.inbox.post(Msg::View(event));
    }

    pub(crate) fn set_muted(&self, muted: bool) {
        self.inbox.post(Msg::Mute(muted));
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        self.inbox.post(Msg::Stop);
        match self.exited.recv_timeout(STOP_TIMEOUT) {
            Ok(result) => {
                if let Err(error) = result {
                    tracing::error!(%error, "player stopped");
                }
                if let Some(thread) = self.thread.take() {
                    let _ = thread.join();
                }
            }
            Err(error) => tracing::error!(%error, "player did not stop cleanly within two seconds"),
        }
    }
}

pub(crate) fn spawn(
    name: String,
    surface: Surface,
    net: tokio::runtime::Handle,
    options: Options,
) -> std::io::Result<Handle> {
    let (tx, rx) = mpsc::channel();
    let inbox = Inbox(tx);
    let (exit_tx, exited) = mpsc::sync_channel(1);
    let outbox = inbox.clone();
    let (provider, loader_events) = Bridge::new();
    let projection = Arc::new(RwLock::new(Projection::default()));
    let stopped = Arc::new(AtomicBool::new(false));
    let thread_stopped = stopped.clone();
    let bridge = PlayerBridge {
        projection: projection.clone(),
        provider,
        loader_events,
    };
    let thread = thread::Builder::new().name(name).spawn(move || {
        let _stopped = StopOnDrop(thread_stopped);
        let result = run(rx, outbox, surface, net, options, bridge);
        if let Err(error) = &result {
            tracing::error!(%error, "player failed");
        }
        let _ = exit_tx.send(result);
    })?;
    Ok(Handle {
        inbox,
        projection,
        stopped,
        exited,
        thread: Some(thread),
    })
}

struct StopOnDrop(Arc<AtomicBool>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

type ReadCallback = Box<dyn FnOnce(Result<&Projection, BridgeError>) + Send>;

pub(crate) struct Read(ReadCallback);

impl Read {
    fn new<R: serde::Serialize + Send + 'static>(
        read: impl FnOnce(&Projection) -> R + Send + 'static,
    ) -> (Read, PendingRead<R>) {
        let (tx, pending) = PendingRead::channel();
        (
            Read(Box::new(move |state| {
                let result = state.and_then(|state| {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read(state)))
                        .map_err(|_| BridgeError::ReadPanicked)
                });
                let _ = tx.send(result);
            })),
            pending,
        )
    }
}

pub(crate) enum Msg {
    Read(Read),
    Command(
        lucent_protocol::command::Command,
        mpsc::Sender<Result<String, BridgeError>>,
    ),
    Call(Call),
    Attach(ScriptEventSender),
    View(ViewEvent),
    Mute(bool),
    Poll(async_task::Runnable),
    Wake,
    Stop,
}

#[derive(Clone)]
pub(crate) struct Inbox(mpsc::Sender<Msg>);

impl Inbox {
    pub(crate) fn call_loader(&self, name: impl Into<String>, args: Vec<Value>) -> PendingCall {
        let (call, pending) = Call::new(name.into(), args);
        if let Err(error) = self.0.send(Msg::Call(call))
            && let Msg::Call(call) = error.0
        {
            call.reject(BridgeError::Stopped);
        }
        pending
    }

    pub(crate) fn read_projection<R: serde::Serialize + Send + 'static>(
        &self,
        read: impl FnOnce(&Projection) -> R + Send + 'static,
    ) -> PendingRead<R> {
        let (read, pending) = Read::new(read);
        self.post(Msg::Read(read));
        pending
    }

    pub(crate) fn post(&self, msg: Msg) {
        let _ = self.0.send(msg);
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PlayerError {
    #[error("couldn't create the renderer: {0}")]
    Renderer(String),
    #[error("couldn't parse the loader: {0}")]
    Loader(String),
    #[error("couldn't start the network backend: {0}")]
    Net(#[from] net::NetError),
}

fn run(
    inbox: mpsc::Receiver<Msg>,
    outbox: Inbox,
    surface: Surface,
    net: tokio::runtime::Handle,
    options: Options,
    bridge: PlayerBridge,
) -> Result<(), PlayerError> {
    let Some((viewport, muted, script_events)) = first_viewport(&inbox, options.muted) else {
        return Ok(());
    };
    let mut running = Running::build(viewport, &surface, Net::new(net)?, outbox, options, bridge)?;
    running.script_events = script_events;
    running.set_muted(muted);
    let mut wait = Duration::ZERO;
    'running: loop {
        let frame_deadline = Instant::now() + wait;
        match inbox.recv_timeout(wait) {
            Ok(msg) => {
                if running.handle(msg).is_break() {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        let drain_deadline = frame_deadline.max(Instant::now() + MIN_INBOX_DRAIN);
        while Instant::now() < drain_deadline {
            let Ok(msg) = inbox.try_recv() else {
                break;
            };
            if running.handle(msg).is_break() {
                break 'running;
            }
        }
        wait = running.advance();
    }
    running.tasks.borrow_mut().clear();
    while let Ok(msg) = inbox.try_recv() {
        if let Msg::Poll(task) = msg {
            task.run();
        }
    }
    Ok(())
}

fn first_viewport(
    inbox: &mpsc::Receiver<Msg>,
    mut muted: bool,
) -> Option<(Viewport, bool, Option<ScriptEventSender>)> {
    let mut script_events = None;
    loop {
        match inbox.recv().ok()? {
            Msg::View(ViewEvent::Resized(viewport)) => {
                return Some((viewport, muted, script_events));
            }
            Msg::Mute(mute) => muted = mute,
            Msg::Attach(producer) => script_events = Some(producer),
            Msg::Call(call) => call.reject(BridgeError::NotReady),
            Msg::Read(read) => (read.0)(Err(BridgeError::NotReady)),
            Msg::Command(_, reply) => {
                let _ = reply.send(Err(BridgeError::NotReady));
            }
            Msg::Stop => return None,
            Msg::View(_) | Msg::Poll(_) | Msg::Wake => {}
        }
    }
}

struct PlayerBridge {
    projection: Arc<RwLock<Projection>>,
    provider: Bridge,
    loader_events: Events,
}

struct Running {
    settings: Arc<RwLock<GameSettings>>,
    settings_applied: bool,
    loader_events: Events,
    script_events: Option<ScriptEventSender>,
    projection: Arc<RwLock<Projection>>,
    player: Arc<Mutex<Player>>,
    tasks: Tasks,
    viewport: Viewport,
    last_tick: Instant,
    effects: Vec<Effect>,
    drawn: u32,
    stats_since: Instant,
    surface: Surface,
}

impl Running {
    fn build(
        viewport: Viewport,
        surface: &Surface,
        net: Net,
        inbox: Inbox,
        options: Options,
        bridge: PlayerBridge,
    ) -> Result<Running, PlayerError> {
        let PlayerBridge {
            projection,
            provider,
            loader_events,
        } = bridge;
        let renderer = surface.renderer(viewport)?;
        let movie = SwfMovie::from_data(
            &options.movie.0,
            "file:///lucent/loader.swf".into(),
            None,
            None,
        )
        .map_err(|error| PlayerError::Loader(error.to_string()))?;
        let tasks = Tasks::default();
        let player = PlayerBuilder::new()
            .with_boxed_renderer(renderer)
            .with_navigator(Navigator::new(net, inbox, tasks.clone()))
            .with_boxed_audio((options.audio)())
            .with_ui(Ui::new(options.clipboard))
            .with_external_interface(Box::new(provider))
            .with_viewport_dimensions(viewport.width(), viewport.height(), viewport.scale())
            .with_autoplay(true)
            .with_letterbox(Letterbox::On)
            .with_movie(movie)
            .build();
        let now = Instant::now();
        Ok(Running {
            settings: options.settings,
            settings_applied: false,
            loader_events,
            script_events: None,
            projection,
            player,
            tasks,
            viewport,
            last_tick: now,
            effects: Vec::new(),
            drawn: 0,
            stats_since: now,
            surface: surface.share(),
        })
    }

    fn handle(&mut self, msg: Msg) -> ControlFlow<()> {
        self.drain_events();
        match msg {
            Msg::Stop => return ControlFlow::Break(()),
            Msg::Attach(producer) => self.script_events = Some(producer),
            Msg::Read(read) => (read.0)(Ok(&self.projection.read())),
            Msg::Command(command, reply) => {
                let result = self.command(command);
                let _ = reply.send(result);
            }
            Msg::Mute(muted) => self.set_muted(muted),
            Msg::Call(mut call) => {
                let result = call.dispatch(&mut self.player.lock().unwrap());
                self.drain_events();
                call.resolve(result);
            }
            Msg::Poll(task) => {
                task.run();
            }
            Msg::Wake => {}
            Msg::View(event) => {
                view::translate(event, self.viewport, &mut self.effects);
                let mut effects = std::mem::take(&mut self.effects);
                for effect in effects.drain(..) {
                    let mut player = self.player.lock().unwrap();
                    match effect {
                        Effect::Event(event) => {
                            player.handle_event(event);
                        }
                        Effect::MouseInStage(inside) => player.set_mouse_in_stage(inside),
                        Effect::Resize(viewport) => {
                            if viewport != self.viewport {
                                player.set_viewport_dimensions(viewport.dimensions());
                                self.viewport = viewport;
                            }
                        }
                    }
                    drop(player);
                    self.drain_events();
                }
                self.effects = effects;
            }
        }
        self.drain_events();
        ControlFlow::Continue(())
    }

    fn set_muted(&self, muted: bool) {
        self.player
            .lock()
            .unwrap()
            .set_volume(if muted { 0.0 } else { 1.0 });
    }

    fn advance(&mut self) -> Duration {
        let now = Instant::now();
        let dt = now - self.last_tick;
        self.last_tick = now;
        let mut player = self.player.lock().unwrap();
        let before = player.frames_run();
        player.tick(FloatDuration::from_std(dt));
        drop(player);
        self.drain_events();
        let mut player = self.player.lock().unwrap();
        if player.frames_run() != before && player.needs_render() && self.surface.wants_frame() {
            player.render();
            self.drawn += 1;
        }
        let elapsed = now.duration_since(self.stats_since);
        if elapsed >= Duration::from_secs(1) {
            tracing::debug!(
                "fps={} viewport={}x{}@{}",
                (f64::from(self.drawn) / elapsed.as_secs_f64()).round() as u32,
                self.viewport.width(),
                self.viewport.height(),
                self.viewport.scale()
            );
            self.drawn = 0;
            self.stats_since = now;
        }
        let wait = player.time_til_next_frame();
        drop(player);
        self.drain_events();
        wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running() -> Running {
        let now = Instant::now();
        let (_provider, loader_events) = Bridge::new();
        Running {
            settings: Arc::new(RwLock::new(GameSettings::default())),
            settings_applied: false,
            loader_events,
            script_events: None,
            projection: Arc::new(RwLock::new(Projection::default())),
            player: PlayerBuilder::new().build(),
            tasks: Tasks::default(),
            viewport: Viewport::new(1024.0, 768.0, 1.0).unwrap(),
            last_tick: now,
            effects: Vec::new(),
            drawn: 0,
            stats_since: now,
            surface: Surface::textures(surface::noop_gpu(), || {}),
        }
    }

    #[test]
    fn an_attachment_before_the_viewport_is_kept_until_the_player_stops() {
        let viewport = Viewport::new(1024.0, 768.0, 1.0).unwrap();
        let (producer, events) = crate::bridge::script_events(1);
        let (tx, rx) = mpsc::channel();
        tx.send(Msg::Attach(producer)).unwrap();
        tx.send(Msg::View(ViewEvent::Resized(viewport))).unwrap();
        let (received, muted, producer) = first_viewport(&rx, false).unwrap();
        assert_eq!((received, muted), (viewport, false));
        assert_eq!(events.try_recv(), Ok(None));
        producer
            .as_ref()
            .unwrap()
            .send(crate::bridge::ScriptEvent::Invalidated);
        assert_eq!(
            events.try_recv(),
            Ok(Some(crate::bridge::ScriptEvent::Invalidated))
        );
        drop(producer);
        assert_eq!(events.try_recv(), Err(BridgeError::Stopped));
    }

    #[test]
    fn stopping_before_the_viewport_ends_an_attachment() {
        let (producer, events) = crate::bridge::script_events(1);
        let (tx, rx) = mpsc::channel();
        tx.send(Msg::Stop).unwrap();
        tx.send(Msg::Attach(producer)).unwrap();
        assert!(first_viewport(&rx, false).is_none());
        drop(rx);
        assert_eq!(events.try_recv(), Err(BridgeError::Stopped));
    }

    #[test]
    fn calls_before_the_first_viewport_are_rejected() {
        let (tx, rx) = mpsc::channel();
        let (call, pending) = Call::new("player.rest".into(), vec![]);
        tx.send(Msg::Call(call)).unwrap();
        tx.send(Msg::Stop).unwrap();
        assert!(first_viewport(&rx, false).is_none());
        assert_eq!(
            pending.recv_timeout(Duration::ZERO),
            Err(BridgeError::NotReady)
        );
    }

    #[test]
    fn closing_the_player_inbox_resolves_queued_calls() {
        let (tx, rx) = mpsc::channel();
        let (call, pending) = Call::new("player.rest".into(), vec![]);
        tx.send(Msg::Call(call)).unwrap();
        drop(rx);
        assert_eq!(
            pending.recv_timeout(Duration::ZERO),
            Err(BridgeError::Stopped)
        );
    }

    #[test]
    fn muting_silences_a_player_until_it_is_unmuted() {
        let mut running = running();
        let _ = running.handle(Msg::Mute(true));
        assert_eq!(running.player.lock().unwrap().volume(), 0.0);
        let _ = running.handle(Msg::Mute(false));
        assert_eq!(running.player.lock().unwrap().volume(), 1.0);
    }

    #[test]
    fn a_mute_sent_before_the_first_viewport_is_kept() {
        let viewport = Viewport::new(1024.0, 768.0, 1.0).unwrap();
        let (tx, rx) = mpsc::channel();
        tx.send(Msg::Mute(true)).unwrap();
        tx.send(Msg::View(ViewEvent::Resized(viewport))).unwrap();
        assert_eq!(
            first_viewport(&rx, false).map(|(viewport, muted, _)| (viewport, muted)),
            Some((viewport, true))
        );
    }

    #[test]
    fn a_player_spawned_for_a_muted_session_starts_muted() {
        let viewport = Viewport::new(1024.0, 768.0, 1.0).unwrap();
        let (tx, rx) = mpsc::channel();
        tx.send(Msg::View(ViewEvent::Resized(viewport))).unwrap();
        assert_eq!(
            first_viewport(&rx, true).map(|(viewport, muted, _)| (viewport, muted)),
            Some((viewport, true))
        );
    }
}
