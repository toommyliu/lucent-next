mod game_window;

use lucent_host::{Host, HostError};
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), HostError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let host = Host::new()?;
    let app = gpui_platform::application();
    let reopen_host = host.clone();
    app.on_reopen(move |cx| {
        if let Err(error) = game_window::open(&reopen_host, cx) {
            tracing::error!(%error, "couldn't reopen a game window");
        }
    });
    app.run(move |cx| {
        if let Err(error) = game_window::open(&host, cx) {
            tracing::error!(%error, "couldn't open a game window");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
    Ok(())
}
