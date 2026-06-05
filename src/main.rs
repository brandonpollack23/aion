#![allow(dead_code, unused_variables)]

mod api;
mod app;
mod auth;
mod config;
mod db;
mod domain;
mod keybinds;
mod notifications;
mod reminders;
mod state;
mod sync;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Panic hook: restore terminal then print a clean error screen
    std::panic::set_hook(Box::new(|info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
        );
        eprintln!();
        eprintln!("⚠  aion crashed");
        eprintln!("───────────────────────────────────────");
        if let Some(loc) = info.location() {
            eprintln!("  at {}:{}", loc.file(), loc.line());
        }
        if let Some(msg) = info.payload().downcast_ref::<&str>() {
            eprintln!("  {}", msg);
        } else if let Some(msg) = info.payload().downcast_ref::<String>() {
            eprintln!("  {}", msg);
        } else {
            eprintln!("  (no message)");
        }
        eprintln!("───────────────────────────────────────");
        eprintln!("  Log: ~/.local/share/aion/logs/");
        eprintln!();
    }));

    // Initialize file-based tracing
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.local/share"))
        .join("aion")
        .join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "aion.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    app::run().await
}
