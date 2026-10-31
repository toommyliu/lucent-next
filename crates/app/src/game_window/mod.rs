#[cfg(feature = "overlay-check")]
mod overlay_check;
mod player_view;

use gpui::{
    App, AppContext as _, Bounds, Context, Entity, Focusable as _, IntoElement, ParentElement as _,
    Render, Styled as _, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowOptions, div, px, rgb, size,
};
use lucent_host::Host;
use lucent_host::ScriptRequest;

use player_view::{PlayerView, PlayerViewError};

const WINDOW_WIDTH: f32 = 1024.;
const WINDOW_HEIGHT: f32 = 798.;
const TAB_STRIP_HEIGHT: f32 = 30.;

pub fn open(
    host: &Host,
    script: Option<ScriptRequest>,
    cx: &mut App,
) -> anyhow::Result<WindowHandle<GameWindow>> {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some("Lucent".into()),
            appears_transparent: true,
            ..Default::default()
        }),
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    cx.open_window(options, |window, cx| {
        cx.new(|cx| GameWindow::new(host, script, window, cx))
    })
}

pub struct GameWindow {
    player: Result<Entity<PlayerView>, PlayerViewError>,
}

impl GameWindow {
    fn new(
        host: &Host,
        script: Option<ScriptRequest>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> GameWindow {
        let player = PlayerView::new(host, script, window, cx);
        match &player {
            Ok(view) => window.focus(&view.focus_handle(cx), cx),
            Err(error) => tracing::error!(%error, "no player view"),
        }
        GameWindow { player }
    }
}

impl Render for GameWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let game_area = match &self.player {
            Ok(view) => div().flex_1().min_h_0().child(view.clone()),
            Err(error) => div()
                .flex_1()
                .bg(rgb(0x000000))
                .text_color(rgb(0xffffff))
                .child(error.to_string()),
        };
        #[cfg(feature = "overlay-check")]
        let game_area = game_area.relative().children(overlay_check::overlay());
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .h(px(TAB_STRIP_HEIGHT))
                    .w_full()
                    .bg(rgb(0x1f1f1f)),
            )
            .child(game_area)
    }
}
