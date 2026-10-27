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
    gpui_platform::application().run(move |cx| {
        if let Err(error) = game_window::open(&host, cx) {
            tracing::error!(%error, "couldn't open a game window");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
    Ok(())
}
