mod app;
mod ui;
mod ws;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[derive(Parser)]
#[command(name = "oxide-tui", version, about = "OXIDE C2 Operator Interface")]
struct Args {
    /// Server WebSocket address
    #[arg(short, long, default_value = "ws://127.0.0.1:7878/ws")]
    server: String,

    /// Auth token (auto-read from oxide-server.toml if omitted)
    #[arg(short, long)]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let token = match args.token {
        Some(t) => t,
        None => read_token_from_toml()
            .ok_or_else(|| anyhow::anyhow!(
                "No --token given and could not read oxide-server.toml. \
                 Run oxide-server first or pass --token <token>."
            ))?,
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let res = run(&mut term, &args.server, &token).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e}");
    }
    Ok(())
}

fn read_token_from_toml() -> Option<String> {
    let content = std::fs::read_to_string("oxide-server.toml").ok()?;
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("operator_token") {
            let rest = rest.trim_start_matches([' ', '=', '"']).trim_end_matches('"');
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

async fn run<B: ratatui::backend::Backend>(
    term:  &mut Terminal<B>,
    addr:  &str,
    token: &str,
) -> Result<()> {
    let (ws_tx, mut ws_rx) = ws::connect(addr, token).await?;
    let mut app = app::App::new(ws_tx);

    // Initial fetch
    app.send_op(oxide_common::protocol::OpMessage::ListAgents).await;
    app.send_op(oxide_common::protocol::OpMessage::ListListeners).await;
    app.send_op(oxide_common::protocol::OpMessage::ListLoot).await;

    let mut event_stream = crossterm::event::EventStream::new();
    use futures::StreamExt;

    loop {
        term.draw(|f| ui::draw(f, &mut app))?;

        tokio::select! {
            // Keyboard / mouse events
            Some(Ok(evt)) = event_stream.next() => {
                if app.handle_event(evt).await? {
                    break; // quit signal
                }
            }
            // Server messages
            Some(msg) = ws_rx.recv() => {
                app.handle_server_message(msg);
            }
        }
    }

    Ok(())
}
