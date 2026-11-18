mod agents;
mod dashboard;
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

// ── Colour palette ────────────────────────────────────────────────────────────
pub const C_ACCENT:  Color = Color::Cyan;
pub const C_SELECT:  Color = Color::LightRed;
pub const C_LABEL:   Color = Color::Yellow;
pub const C_OK:      Color = Color::Green;
pub const C_WARN:    Color = Color::Yellow;
pub const C_DANGER:  Color = Color::Red;
pub const C_DIM:     Color = Color::DarkGray;
pub const C_FG:      Color = Color::White;
pub const C_BG:      Color = Color::Black;
pub const C_BORDER:  Color = Color::DarkGray;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Fill entire frame with black — prevents compile-output bleed-through
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(C_BG)), area);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    draw_tabs(f, app, root[0]);

    match app.tab {
        Tab::Dashboard => dashboard::draw(f, app, root[1]),
        Tab::Agents    => agents::draw(f, app, root[1]),
        Tab::Listeners => listeners::draw(f, app, root[1]),
        Tab::Sessions  => sessions::draw(f, app, root[1]),
        Tab::Loot      => loot::draw(f, app, root[1]),
        Tab::Build     => draw_build(f, app, root[1]),
    }

    draw_statusbar(f, app, root[2]);

    if let Some(modal) = &app.modal.clone() {
        modals::draw(f, app, modal, area);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    use oxide_common::types::ListenerStatus;

    let agents_n  = app.agents.len();
    let listen_n  = app.listeners.iter().filter(|l| l.status == ListenerStatus::Running).count();
    let loot_n    = app.loot.len();

    let badge = |n: usize| -> String {
        if n > 0 { format!(" ({n})") } else { String::new() }
    };

    let tabs: Vec<(usize, String)> = vec![
        (0, " 1 Dashboard ".into()),
        (1, format!(" 2 Agents{} ", badge(agents_n))),
        (2, format!(" 3 Listeners{} ", badge(listen_n))),
        (3, " 4 Sessions ".into()),
        (4, format!(" 5 Loot{} ", badge(loot_n))),
        (5, " 6 Build ".into()),
    ];

    let active_i    = app.tab.index();
    let active_sty  = Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let inactive_sty = Style::default().fg(C_DIM);
    let sep_sty      = Style::default().fg(C_BORDER);

    let (conn_sym, conn_sty) = if app.ws_connected {
        ("● ", Style::default().fg(C_OK))
    } else {
        ("○ ", Style::default().fg(C_DANGER).add_modifier(Modifier::BOLD))
    };

    let mut spans: Vec<Span> = vec![
        Span::styled("  ◈ OXIDE  ", Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)),
        Span::styled(conn_sym, conn_sty),
        Span::styled("│", sep_sty),
    ];

    for (idx, name) in &tabs {
        spans.push(Span::styled(
            name.clone(),
            if *idx == active_i { active_sty } else { inactive_sty },
        ));
        spans.push(Span::styled("│", sep_sty));
    }

    let header = Paragraph::new(Line::from(spans))
        .block(Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG)));
    f.render_widget(header, area);
}

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let agents    = app.agents.len();
    let active    = app.agents.iter().filter(|a| a.status == oxide_common::types::AgentStatus::Active).count();
    let listeners = app.listeners.iter().filter(|l| l.status == oxide_common::types::ListenerStatus::Running).count();

    let filter_hint = if app.filter_active {
        format!(" /{}_", app.filter)
    } else if !app.filter.is_empty() {
        format!(" /{}", app.filter)
    } else {
        String::new()
    };

    let (srv_dot, srv_sty) = if app.ws_connected {
        ("● server  ", Style::default().fg(C_OK))
    } else {
        ("○ DISCONNECTED  ", Style::default().fg(C_DANGER).add_modifier(Modifier::BOLD))
    };

    let line = Line::from(vec![
        Span::styled(srv_dot, srv_sty),
        Span::styled(format!("{active}/{agents}"), Style::default().fg(C_OK).add_modifier(Modifier::BOLD)),
        Span::styled(" agents  ", Style::default().fg(C_DIM)),
        Span::styled("◉ ", Style::default().fg(C_ACCENT)),
        Span::styled(format!("{listeners}"), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" listeners  ", Style::default().fg(C_DIM)),
        Span::styled("│", Style::default().fg(C_BORDER)),
        Span::styled("  [1-6]", Style::default().fg(C_ACCENT)),
        Span::styled(" tab  ", Style::default().fg(C_DIM)),
        Span::styled("[/]", Style::default().fg(C_ACCENT)),
        Span::styled(" filter  ", Style::default().fg(C_DIM)),
        Span::styled("[q]", Style::default().fg(C_DANGER)),
        Span::styled(" quit", Style::default().fg(C_DIM)),
        Span::styled(filter_hint, Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)),
    ]);

    let para = Paragraph::new(line).style(Style::default().bg(C_BG));
    f.render_widget(para, area);
}

fn draw_build(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::{text::Text, widgets::Paragraph};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .margin(1)
        .split(area);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Press ", Style::default().fg(C_DIM)),
            Span::styled("[G]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" to open the Payload Generator", Style::default().fg(C_FG)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   The generator will cross-compile an ", Style::default().fg(C_DIM)),
            Span::styled("oxide-agent.exe", Style::default().fg(C_ACCENT)),
            Span::styled(" for Windows x64.", Style::default().fg(C_DIM)),
        ]),
    ];

    if let Some(ref status) = app.build_status {
        let color = if status.starts_with("FAILED") { C_DANGER }
                    else if status.starts_with("Done") { C_OK }
                    else { C_WARN };

        let para = Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled("   Status: ", Style::default().fg(C_DIM)),
                Span::styled(status.as_str(), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ]),
        ]))
        .block(Block::default().borders(Borders::ALL)
            .title(" Build Status ")
            .border_style(Style::default().fg(color)));
        f.render_widget(para, chunks[1]);
    }

    let para = Paragraph::new(Text::from(content))
        .block(Block::default().borders(Borders::ALL)
            .title(" Payload Generator ")
            .border_style(Style::default().fg(C_BORDER)));
    f.render_widget(para, chunks[0]);
}
