use color_eyre::eyre::Result;
use tracing_subscriber::EnvFilter;

/// Initialize logging. In TUI mode, logs go to a file. In CLI mode, logs go to stderr.
pub fn init(tui_mode: bool) -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    if tui_mode {
        // Log to file in TUI mode to avoid corrupting the terminal
        let log_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("n8sight");
        std::fs::create_dir_all(&log_dir)?;

        let file_appender = tracing_appender::rolling::daily(&log_dir, "n8sight.log");

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(file_appender)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .init();
    }

    Ok(())
}
