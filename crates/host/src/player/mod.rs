mod backends;
pub(crate) mod frames;
mod net;
pub(crate) mod surface;
pub(crate) mod view;

use std::ops::ControlFlow;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ruffle_core::{FloatDuration, Player, PlayerBuilder, config::Letterbox, tag_utils::SwfMovie};

pub(crate) use backends::Clipboard;
pub(crate) use surface::Surface;
pub(crate) use view::{ViewEvent, Viewport};

use backends::{Navigator, Tasks, Ui, audio};
use net::Net;
use view::Effect;

const STOP_TIMEOUT: Duration = Duration::from_secs(2);
const LOADER_SWF: &[u8] = include_bytes!("../../../../assets/loader.swf");
const LOADER_URL: &str = "https://game.aq.com/game/loader.swf";

pub(crate) struct Handle {
    inbox: Inbox,
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
    clipboard: Clipboard,
    muted: bool,
) -> std::io::Result<Handle> {
    let (tx, rx) = mpsc::channel();
    let inbox = Inbox(tx);
    let (exit_tx, exited) = mpsc::sync_channel(1);
    let outbox = inbox.clone();
    let thread = thread::Builder::new().name(name).spawn(move || {
        let result = run(rx, outbox, surface, net, clipboard, muted);
        if let Err(error) = &result {
            tracing::error!(%error, "player failed");
        }
        let _ = exit_tx.send(result);
    })?;
    Ok(Handle {
        inbox,
        exited,
        thread: Some(thread),
    })
}

pub(crate) enum Msg {
    View(ViewEvent),
    Mute(bool),
    Poll(async_task::Runnable),
    Wake,
    Stop,
}

#[derive(Clone)]
pub(crate) struct Inbox(mpsc::Sender<Msg>);

impl Inbox {
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
    clipboard: Clipboard,
    muted: bool,
) -> Result<(), PlayerError> {
    let Some((viewport, muted)) = first_viewport(&inbox, muted) else {
        return Ok(());
    };
    let mut running = Running::build(viewport, &surface, Net::new(net)?, outbox, clipboard)?;
    running.set_muted(muted);
    let mut wait = Duration::ZERO;
    'running: loop {
        match inbox.recv_timeout(wait) {
            Ok(msg) => {
                if running.handle(msg).is_break() {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        while let Ok(msg) = inbox.try_recv() {
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

fn first_viewport(inbox: &mpsc::Receiver<Msg>, mut muted: bool) -> Option<(Viewport, bool)> {
    loop {
        match inbox.recv().ok()? {
            Msg::View(ViewEvent::Resized(viewport)) => return Some((viewport, muted)),
            Msg::Mute(mute) => muted = mute,
            Msg::Stop => return None,
            Msg::View(_) | Msg::Poll(_) | Msg::Wake => {}
        }
    }
}

struct Running {
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
        clipboard: Clipboard,
    ) -> Result<Running, PlayerError> {
        let renderer = surface.renderer(viewport)?;
        let movie = SwfMovie::from_data(LOADER_SWF, LOADER_URL.into(), None, None)
            .map_err(|error| PlayerError::Loader(error.to_string()))?;
        let tasks = Tasks::default();
        let player = PlayerBuilder::new()
            .with_boxed_renderer(renderer)
            .with_navigator(Navigator::new(
                url::Url::parse(LOADER_URL).unwrap(),
                net,
                inbox,
                tasks.clone(),
            ))
            .with_boxed_audio(audio())
            .with_ui(Ui::new(clipboard))
            .with_external_interface(Box::new(crate::bridge::Bridge))
            .with_viewport_dimensions(viewport.width(), viewport.height(), viewport.scale())
            .with_autoplay(true)
            .with_letterbox(Letterbox::On)
            .with_movie(movie)
            .build();
        let now = Instant::now();
        Ok(Running {
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
        match msg {
            Msg::Stop => return ControlFlow::Break(()),
            Msg::Mute(muted) => self.set_muted(muted),
            Msg::Poll(task) => {
                task.run();
            }
            Msg::Wake => {}
            Msg::View(event) => {
                view::translate(event, self.viewport, &mut self.effects);
                let mut player = self.player.lock().unwrap();
                for effect in self.effects.drain(..) {
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
                }
            }
        }
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
        player.time_til_next_frame()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running() -> Running {
        let now = Instant::now();
        Running {
            player: PlayerBuilder::new().build(),
            tasks: Tasks::default(),
            viewport: Viewport::new(1024.0, 768.0, 1.0).unwrap(),
            last_tick: now,
            effects: Vec::new(),
            drawn: 0,
            stats_since: now,
            surface: Surface::textures(surface::tests::noop_gpu(), || {}),
        }
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
        assert_eq!(first_viewport(&rx, false), Some((viewport, true)));
    }

    #[test]
    fn a_player_spawned_for_a_muted_session_starts_muted() {
        let viewport = Viewport::new(1024.0, 768.0, 1.0).unwrap();
        let (tx, rx) = mpsc::channel();
        tx.send(Msg::View(ViewEvent::Resized(viewport))).unwrap();
        assert_eq!(first_viewport(&rx, true), Some((viewport, true)));
    }
}
