use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
use tracing_subscriber::filter::{LevelFilter, filter_fn};

/// Custom timer that emits local wall-clock time with the UTC offset
/// (including DST), e.g. `2026-05-25T14:01:06.301333+02:00`.
struct LocalTime;

impl tracing_subscriber::fmt::time::FormatTime for LocalTime {
    fn format_time(
        &self,
        w: &mut tracing_subscriber::fmt::format::Writer<'_>,
    ) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.6f%:z"))
    }
}

/// Initialise application logging.
///
/// * `log_dir` — when `Some`, writes two daily rolling log files:
///   - `mycela.log.<YYYY-MM-DD>` — INFO, WARN and ERROR (operational log)
///   - `mycela.debug.<YYYY-MM-DD>` — TRACE and DEBUG (verbose/diagnostic log)
///   The returned guards **must** be held for the entire process lifetime.
/// * When `None`, logs go to stdout only (DEBUG and above).
///
/// Log level is controlled by `RUST_LOG`; default is `mycela=trace`.
/// All timestamps reflect local time including the DST offset.
pub fn init_logging(
    log_dir: Option<&std::path::Path>,
) -> Vec<tracing_appender::non_blocking::WorkerGuard> {
    // Global gate — set to TRACE so per-layer filters can route events freely.
    // Default: full trace for mycela internals; all other crates (including the
    // calling binary) pass at INFO and above so operational messages are visible.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,mycela=trace,tower_http=debug,axum=debug".into());

    // Console: DEBUG and above with ANSI colour.
    let console_layer = tracing_subscriber::fmt::layer()
        .with_timer(LocalTime)
        .with_filter(LevelFilter::DEBUG);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    let mut guards: Vec<tracing_appender::non_blocking::WorkerGuard> = Vec::new();

    if let Some(dir) = log_dir {
        std::fs::create_dir_all(dir).ok();

        let app_name = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "app".to_string());

        // Operational log — INFO, WARN, ERROR.
        let info_appender = tracing_appender::rolling::daily(dir, format!("{app_name}.log"));
        let (info_nb, info_guard) = tracing_appender::non_blocking(info_appender);
        let info_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_timer(LocalTime)
            .with_writer(info_nb)
            .with_filter(LevelFilter::INFO);

        // Verbose log — TRACE and DEBUG only.
        let debug_appender = tracing_appender::rolling::daily(dir, format!("{app_name}.debug"));
        let (debug_nb, debug_guard) = tracing_appender::non_blocking(debug_appender);
        let debug_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_timer(LocalTime)
            .with_writer(debug_nb)
            .with_filter(filter_fn(|m| {
                matches!(*m.level(), tracing::Level::TRACE | tracing::Level::DEBUG)
            }));

        registry.with(info_layer).with(debug_layer).init();
        guards.push(info_guard);
        guards.push(debug_guard);
    } else {
        registry.init();
    }

    guards
}
