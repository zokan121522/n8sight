mod action;
mod app;
mod cli;
mod cli_worker;
mod client;
mod config;
mod domain;
mod event;
mod logging;
mod scroll_state;
mod tui;
mod widgets;

use std::sync::Arc;

use clap::Parser;
use color_eyre::eyre::Result;
use tokio::sync::mpsc;
use tracing::warn;

use crate::action::Effect;
use crate::client::N8nClient;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let _ = dotenvy::dotenv();

    let cli = cli::Cli::parse();

    let cfg = config::Config::load(
        cli.url.as_deref(),
        cli.api_key.as_deref(),
        cli.project.as_deref(),
    )?;

    logging::init(true)?;

    let client: Arc<dyn N8nClient> = if cli.mock {
        Arc::new(client::mock::MockN8nClient::new())
    } else {
        cfg.validate()?;
        Arc::new(client::http::HttpN8nClient::new(&cfg)?)
    };

    // Channels
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let (worker_tx, worker_rx) = mpsc::unbounded_channel();

    // Start the CLI worker
    let worker = cli_worker::CliWorker::new(Arc::clone(&client), worker_rx, action_tx.clone());
    tokio::spawn(worker.run());

    // Start the event handler
    let event_handler = event::EventHandler::new(action_tx.clone());
    event_handler.start();

    // CLI --refresh overrides config file and env var
    let refresh_interval = cli.refresh.unwrap_or(cfg.refresh_interval_secs);

    // Initialize
    let mut terminal = tui::Tui::new()?;
    let mut app = app::App::new(cfg.api_url.clone(), refresh_interval);

    // Process startup effects
    let init_effects = app::App::init_effects();
    process_effects(init_effects, &worker_tx);

    // Main loop
    loop {
        app.tick_frame();

        terminal
            .terminal()
            .draw(|frame| widgets::render(&mut app, frame))?;

        if let Some(action) = action_rx.recv().await {
            let effects = app.update(action);
            process_effects(effects, &worker_tx);
        }

        if app.should_quit {
            break;
        }
    }

    terminal.restore()?;
    Ok(())
}

fn process_effects(
    effects: Vec<Effect>,
    worker_tx: &mpsc::UnboundedSender<cli_worker::WorkerRequest>,
) {
    for effect in effects {
        match effect {
            Effect::SendWorkerRequest(req) => {
                if let Err(e) = worker_tx.send(req) {
                    warn!("Failed to send worker request: {}", e);
                }
            }
            Effect::CopyToClipboard(text) => {
                if let Err(e) = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&text)) {
                    warn!("Failed to copy to clipboard: {}", e);
                }
            }
            Effect::OpenUrl(url) => {
                if let Err(e) = open::that(&url) {
                    warn!("Failed to open URL: {}", e);
                }
            }
            Effect::Quit => {}
        }
    }
}
