mod app;
mod handlers;
mod config;
mod log;
mod state;
mod ui;
mod cache;
mod lrc;


use std::sync::Arc;

use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use sonus_core::api::YtmClient;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        );
        let _ = crossterm::terminal::disable_raw_mode();
        eprintln!("Application panicked: {:?}", info);
    }));

    let ytm = YtmClient::new().await;
    let ytm = Arc::new(ytm);

    let (player_cmd_tx, player_cmd_rx) = std::sync::mpsc::channel();
    let (player_evt_tx, player_evt_rx) = mpsc::unbounded_channel();
    let evt_tx_clone = player_evt_tx.clone();
    sonus_core::player::spawn(player_cmd_rx, player_evt_tx);

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = app::App::new();
    let result = app
        .run(&mut terminal, player_cmd_tx, player_evt_rx, evt_tx_clone, ytm)
        .await;

    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    crossterm::terminal::disable_raw_mode()?;

    result
}
