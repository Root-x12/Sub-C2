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
use oxide_common::protocol::OpMessage;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[derive(Parser)]
#[command(name = "oxide-tui", version, about = "OXIDE C2 Operator Interface")]
struct Args {
    #[arg(short, long, default_value = "ws://127.0.0.1:7878/ws")]
    server: String,
    #[arg(short, long)]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let token = match args.token {
        Some(t) => t,
        None => read_token_from_toml().ok_or_else(|| {
            anyhow::anyhow!(
                "No --token given and could not read oxide-server.toml. \
                 Run oxide-server first or pass --token <token>."
            )
        })?,
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let res = run(&mut term, &args.server, &token).await;

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
    // Initial connect
    let (ws_tx, mut ws_rx) = connect_with_retry(addr, token).await;
    let mut app = app::App::new(ws_tx);
    app.ws_connected = true;

    // Initialize tachyonfx effect engine
    let mut fx_engine = ui::effects::FxEngine::new();

    initial_fetch(&app).await;

    let mut event_stream = crossterm::event::EventStream::new();

    // ── Frame rate: 30fps for smooth tachyonfx animations ────────────
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(33));

    // Tick counter for beacon history (every ~1 second = 30 frames)
    let mut frame_count: u64 = 0;

    use futures::StreamExt;

    loop {
        term.draw(|f| ui::draw(f, &mut app, &mut fx_engine))?;

        tokio::select! {
            Some(Ok(evt)) = event_stream.next() => {
                if app.handle_event(evt).await? {
                    break;
                }
            }

            msg = ws_rx.recv() => {
                match msg {
                    Some(msg) => {
                        app.handle_server_message(msg);
                    }
                    None => {
                        // WebSocket dropped — reconnect
                        app.ws_connected = false;
                        app.push_log("⚠ server disconnected — reconnecting…");
                        term.draw(|f| ui::draw(f, &mut app, &mut fx_engine))?;

                        let (new_tx, new_rx) = connect_with_retry(addr, token).await;
                        ws_rx = new_rx;
                        app.ws_tx = new_tx;
                        app.ws_connected = true;
                        app.push_log("✓ reconnected to server");
                        initial_fetch(&app).await;
                    }
                }
            }

            _ = ticker.tick() => {
                frame_count += 1;
                // Run app tick logic every ~30 frames (~1 second)
                if frame_count % 30 == 0 {
                    app.tick();
                }
            }
        }
    }

    Ok(())
}

async fn connect_with_retry(addr: &str, token: &str) -> (tokio::sync::mpsc::Sender<OpMessage>, tokio::sync::mpsc::Receiver<oxide_common::protocol::SrvMessage>) {
    loop {
        match ws::connect(addr, token).await {
            Ok(pair) => return pair,
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
}

async fn initial_fetch(app: &app::App) {
    app.send_op(OpMessage::ListAgents).await;
    app.send_op(OpMessage::ListListeners).await;
    app.send_op(OpMessage::ListLoot).await;
}
