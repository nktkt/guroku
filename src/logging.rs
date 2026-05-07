use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialise tracing once at startup. Reads `GUROKU_LOG` (or `RUST_LOG`),
/// defaulting to `info`.
pub fn init() {
    let filter = EnvFilter::try_from_env("GUROKU_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().without_time().with_target(false))
        .try_init();
}
