//! The main entrypoint.

use color_eyre::Result;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

mod app;
mod cli_args;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    setup_logging();
    app::App::run().await
}

/// Setup logging.
fn setup_logging() {
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "off".to_owned());
    let env_filter = tracing_subscriber::EnvFilter::new(log_level);
    let layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    tracing_subscriber::registry()
        .with(env_filter)
        .with(layer)
        .init();
}
