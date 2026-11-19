mod agents;
mod dashboard;
pub mod effects;
mod graph;
mod listeners;
mod loot;
mod modals;
mod sessions;

use crate::app::{App, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

// ── Colour palette — cyberpunk / hacker theme ────────────────────────────────
// Deep background tones
pub const C_BG:      Color = Color::Rgb(10, 10, 18);
pub const C_BG_ALT:  Color = Color::Rgb(16, 18, 30);     // slightly lighter panels
pub const C_SURFACE: Color = Color::Rgb(22, 24, 38);     // card/modal surface

// Borders & dim text
pub const C_BORDER:  Color = Color::Rgb(45, 50, 70);
pub const C_DIM:     Color = Color::Rgb(90, 95, 115);

// Primary text
pub const C_FG:      Color = Color::Rgb(200, 205, 220);

// Accent colours — neon
pub const C_ACCENT:  Color = Color::Rgb(0, 220, 255);     // electric cyan
pub const C_SELECT:  Color = Color::Rgb(255, 70, 100);    // hot pink / red
pub const C_LABEL:   Color = Color::Rgb(180, 140, 255);   // soft purple labels
pub const C_OK:      Color = Color::Rgb(0, 255, 136);     // matrix green
pub const C_WARN:    Color = Color::Rgb(255, 200, 40);    // amber
pub const C_DANGER:  Color = Color::Rgb(255, 50, 50);     // bright red
pub const C_PURPLE:  Color = Color::Rgb(170, 100, 255);   // violet accent
pub const C_BLUE:    Color = Color::Rgb(80, 140, 255);    // info blue

// Status-specific
pub const C_ACTIVE:  Color = Color::Rgb(0, 255, 136);
pub const C_STALE:   Color = Color::Rgb(255, 180, 40);
pub const C_DEAD:    Color = Color::Rgb(100, 50, 50);

pub fn draw(f: &mut Frame, app: &mut App, fx_engine: &mut effects::FxEngine) {
    let area = f.area();

    // Fill entire frame
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(C_BG)), area);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // header / tab bar
            Constraint::Min(0),      // main content
            Constraint::Length(2),   // status bar (2 lines now)
        ])
        .split(area);

    draw_header(f, app, root[0]);

    match app.tab {
        Tab::Dashboard => dashboard::draw(f, app, root[1]),
        Tab::Agents    => agents::draw(f, app, root[1]),
        Tab::Listeners => listeners::draw(f, app, root[1]),
        Tab::Sessions  => sessions::draw(f, app, root[1]),
        Tab::Loot      => loot::draw(f, app, root[1]),
        Tab::Build     => draw_build(f, app, root[1]),
    }

    draw_statusbar(f, app, root[2]);

    // Track modal area for effects
    let modal_area = if let Some(modal) = &app.modal.clone() {
        let popup = modals::draw(f, app, modal, area);
        Some(popup)
    } else {
        None
    };

    // ── Apply tachyonfx effects as post-processing ───────────────────
    fx_engine.process_frame(
        f,
        root[0],           // header area
        root[1],           // content area
        root[2],           // statusbar area
        modal_area,        // modal area (if any)
        app.tab.index(),   // current tab index
        app.modal.is_some(),
    );
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    use oxide_common::types::ListenerStatus;

    let agents_n  = app.agents.len();
    let listen_n  = app.listeners.iter().filter(|l| l.status == ListenerStatus::Running).count();
    let loot_n    = app.loot.len();

    let badge = |n: usize| -> String {
        if n > 0 { format!(" {n}") } else { String::new() }
    };

    let tabs: Vec<(usize, String, String)> = vec![
        (0, "Dashboard".into(), String::new()),
        (1, "Agents".into(),    badge(agents_n)),
        (2, "Listeners".into(), badge(listen_n)),
        (3, "Sessions".into(),  String::new()),
        (4, "Loot".into(),      badge(loot_n)),
        (5, "Build".into(),     String::new()),
    ];

    let active_i = app.tab.index();

    let (conn_sym, conn_sty) = if app.ws_connected {
        ("⚡", Style::default().fg(C_OK).add_modifier(Modifier::BOLD))
    } else {
        ("⊘ ", Style::default().fg(C_DANGER).add_modifier(Modifier::BOLD))
    };

    // Animated branding — pulse the brackets with tick
    let pulse_char = match (app.tick_count / 2) % 4 {
        0 => "◈",
        1 => "◇",
        2 => "◈",
        _ => "◆",
    };

    let mut spans: Vec<Span> = vec![
        Span::styled(" ", Style::default()),
        Span::styled(pulse_char, Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)),
        Span::styled(" OXIDE", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("://", Style::default().fg(Color::Rgb(0, 150, 180))),
        Span::styled("C2", Style::default().fg(Color::Rgb(255, 255, 255)).add_modifier(Modifier::BOLD)),
        Span::styled(" ", Style::default()),
        Span::styled(conn_sym, conn_sty),
        Span::styled("  ", Style::default()),
        Span::styled("▐", Style::default().fg(Color::Rgb(0, 80, 100))),
    ];

    for (idx, name, badge_str) in &tabs {
        let num = idx + 1;
        if *idx == active_i {
            // Active tab: highlighted with accent color underline-style
            spans.push(Span::styled(
                format!(" {num}"),
                Style::default().fg(C_ACCENT),
            ));
            spans.push(Span::styled(
                format!(":{name}"),
                Style::default().fg(Color::Rgb(255, 255, 255)).add_modifier(Modifier::BOLD),
            ));
            if !badge_str.is_empty() {
                spans.push(Span::styled(
                    badge_str.clone(),
                    Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
                ));
            }
            spans.push(Span::styled(" ", Style::default()));
        } else {
            spans.push(Span::styled(
                format!(" {num}"),
                Style::default().fg(Color::Rgb(60, 65, 80)),
            ));
            spans.push(Span::styled(
                format!(":{name}"),
                Style::default().fg(C_DIM),
            ));
            if !badge_str.is_empty() {
                spans.push(Span::styled(
                    badge_str.clone(),
                    Style::default().fg(Color::Rgb(100, 60, 60)),
                ));
            }
            spans.push(Span::styled(" ", Style::default()));
        }
        spans.push(Span::styled("▐", Style::default().fg(Color::Rgb(25, 28, 40))));
    }

    let header = Paragraph::new(Line::from(spans))
        .block(Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Rgb(0, 60, 80)))
            .style(Style::default().bg(C_BG_ALT)));
    f.render_widget(header, area);
}

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let agents    = app.agents.len();
    let active    = app.agents.iter().filter(|a| a.status == oxide_common::types::AgentStatus::Active).count();
    let stale     = app.agents.iter().filter(|a| a.status == oxide_common::types::AgentStatus::Stale).count();
    let listeners = app.listeners.iter().filter(|l| l.status == oxide_common::types::ListenerStatus::Running).count();

    let filter_hint = if app.filter_active {
        format!(" /{}_", app.filter)
    } else if !app.filter.is_empty() {
        format!(" /{}", app.filter)
    } else {
        String::new()
    };

    let (srv_dot, srv_sty) = if app.ws_connected {
        ("▣ LINKED", Style::default().fg(C_OK).add_modifier(Modifier::BOLD))
    } else {
        ("▢ OFFLINE", Style::default().fg(C_DANGER).add_modifier(Modifier::BOLD))
    };

    // Animated separator
    let sep_char = match (app.tick_count / 3) % 4 {
        0 => "┃",
        1 => "│",
        2 => "┃",
        _ => "¦",
    };
    let sep = Span::styled(format!("  {sep_char}  "), Style::default().fg(Color::Rgb(0, 60, 80)));

    // Line 1: stats with cyberpunk formatting
    let line1 = Line::from(vec![
        Span::styled(" ▸ ", Style::default().fg(Color::Rgb(0, 80, 100))),
        Span::styled(srv_dot, srv_sty),
        sep.clone(),
        Span::styled("AGT:", Style::default().fg(Color::Rgb(0, 150, 180))),
        Span::styled(format!("{active}"), Style::default().fg(C_OK).add_modifier(Modifier::BOLD)),
        Span::styled("/", Style::default().fg(C_DIM)),
        Span::styled(format!("{agents}"), Style::default().fg(C_FG)),
        if stale > 0 {
            Span::styled(format!(" ⚠{stale}"), Style::default().fg(C_WARN))
        } else {
            Span::raw("")
        },
        sep.clone(),
        Span::styled("LSN:", Style::default().fg(Color::Rgb(0, 150, 180))),
        Span::styled(format!("{listeners}"), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        sep.clone(),
        Span::styled("[1-6]", Style::default().fg(C_ACCENT)),
        Span::styled("tab ", Style::default().fg(C_DIM)),
        Span::styled("[/]", Style::default().fg(C_ACCENT)),
        Span::styled("find ", Style::default().fg(C_DIM)),
        Span::styled("[l]", Style::default().fg(C_ACCENT)),
        Span::styled("sync ", Style::default().fg(C_DIM)),
        Span::styled("[q]", Style::default().fg(C_DANGER)),
        Span::styled("exit", Style::default().fg(C_DIM)),
        Span::styled(filter_hint, Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)),
    ]);

    let para = Paragraph::new(vec![line1])
        .style(Style::default().bg(Color::Rgb(8, 10, 20)));
    f.render_widget(para, area);
}

fn draw_build(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::Text;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .margin(1)
        .split(area);

    let content = vec![
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("     ╔══════════════════════════════════════════╗", Style::default().fg(C_BORDER)),
        ]),
        Line::from(vec![
            Span::styled("     ║  ", Style::default().fg(C_BORDER)),
            Span::styled("Press ", Style::default().fg(C_DIM)),
            Span::styled("[G]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" to open the Payload Generator", Style::default().fg(C_FG)),
            Span::styled("  ║", Style::default().fg(C_BORDER)),
        ]),
        Line::from(vec![
            Span::styled("     ╚══════════════════════════════════════════╝", Style::default().fg(C_BORDER)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("     Cross-compiles ", Style::default().fg(C_DIM)),
            Span::styled("oxide-agent", Style::default().fg(C_ACCENT)),
            Span::styled(" for ", Style::default().fg(C_DIM)),
            Span::styled("Windows x64", Style::default().fg(C_FG).add_modifier(Modifier::BOLD)),
            Span::styled(" (x86_64-pc-windows-gnu)", Style::default().fg(C_DIM)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("     Supported formats: ", Style::default().fg(C_DIM)),
            Span::styled("EXE", Style::default().fg(C_OK).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(C_BORDER)),
            Span::styled("DLL", Style::default().fg(C_WARN).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(C_BORDER)),
            Span::styled("Shellcode", Style::default().fg(C_PURPLE).add_modifier(Modifier::BOLD)),
        ]),
    ];

    if let Some(ref status) = app.build_status {
        let (color, icon) = if status.starts_with("FAILED") {
            (C_DANGER, "✗")
        } else if status.starts_with("Done") {
            (C_OK, "✓")
        } else {
            (C_WARN, "▸")
        };

        let para = Paragraph::new(Text::from(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(format!("   {icon} "), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(status.as_str(), Style::default().fg(color)),
            ]),
        ]))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(" Build Status ", Style::default().fg(color).add_modifier(Modifier::BOLD)))
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(C_SURFACE)));
        f.render_widget(para, chunks[1]);
    }

    let para = Paragraph::new(Text::from(content))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " ◈ Payload Generator ",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG)));
    f.render_widget(para, chunks[0]);
}
