mod backends;
mod net;
pub(crate) mod surface;
pub(crate) mod view;

use std::ops::ControlFlow;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ruffle_core::{FloatDuration, Player, PlayerBuilder, config::Letterbox, tag_utils::SwfMovie};

pub(crate) use surface::Surface;
pub(crate) use view::{ViewEvent, Viewport};

use backends::Navigator;
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
) -> std::io::Result<Handle> {
    let (tx, rx) = mpsc::channel();
    let inbox = Inbox(tx);
    let (exit_tx, exited) = mpsc::sync_channel(1);
    let outbox = inbox.clone();
    let thread = thread::Builder::new().name(name).spawn(move || {
        let result = run(rx, outbox, surface, net);
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
) -> Result<(), PlayerError> {
    let Some(viewport) = first_viewport(&inbox) else {
        return Ok(());
    };
    let mut running = Running::build(viewport, &surface, Net::new(net)?, outbox)?;
    let mut wait = Duration::ZERO;
    loop {
        match inbox.recv_timeout(wait) {
            Ok(msg) => {
                if running.handle(msg).is_break() {
                    return Ok(());
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
        while let Ok(msg) = inbox.try_recv() {
            if running.handle(msg).is_break() {
                return Ok(());
            }
        }
        wait = running.advance();
    }
}

fn first_viewport(inbox: &mpsc::Receiver<Msg>) -> Option<Viewport> {
    loop {
        match inbox.recv().ok()? {
            Msg::View(ViewEvent::Resized(viewport)) => return Some(viewport),
            Msg::Stop => return None,
            Msg::View(_) | Msg::Poll(_) | Msg::Wake => {}
        }
    }
}

struct Running {
    player: Arc<Mutex<Player>>,
    viewport: Viewport,
    last_tick: Instant,
    effects: Vec<Effect>,
    drawn: u32,
    stats_since: Instant,
}

impl Running {
    fn build(
        viewport: Viewport,
        surface: &Surface,
        net: Net,
        inbox: Inbox,
    ) -> Result<Running, PlayerError> {
        let renderer = surface.renderer(viewport)?;
        let movie = SwfMovie::from_data(LOADER_SWF, LOADER_URL.into(), None, None)
            .map_err(|error| PlayerError::Loader(error.to_string()))?;
        let player = PlayerBuilder::new()
            .with_renderer(renderer)
            .with_navigator(Navigator::new(
                url::Url::parse(LOADER_URL).unwrap(),
                net,
                inbox,
            ))
            .with_external_interface(Box::new(crate::bridge::Bridge))
            .with_viewport_dimensions(viewport.width(), viewport.height(), viewport.scale())
            .with_autoplay(true)
            .with_letterbox(Letterbox::On)
            .with_movie(movie)
            .build();
        let now = Instant::now();
        Ok(Running {
            player,
            viewport,
            last_tick: now,
            effects: Vec::new(),
            drawn: 0,
            stats_since: now,
        })
    }

    fn handle(&mut self, msg: Msg) -> ControlFlow<()> {
        match msg {
            Msg::Stop => return ControlFlow::Break(()),
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

    fn advance(&mut self) -> Duration {
        let now = Instant::now();
        let dt = now - self.last_tick;
        self.last_tick = now;
        let mut player = self.player.lock().unwrap();
        let before = player.frames_run();
        player.tick(FloatDuration::from_std(dt));
        if player.frames_run() != before && player.needs_render() {
            player.render();
            self.drawn += 1;
        }
        if now.duration_since(self.stats_since) >= Duration::from_secs(1) {
            tracing::debug!(
                "fps={} viewport={}x{}@{}",
                self.drawn,
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
