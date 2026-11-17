use crate::app::App;
use crate::ui::{C_ACCENT, C_BG, C_BORDER, C_DIM, C_DANGER, C_FG, C_OK, C_WARN, graph};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    // ┌──────────────────────┬────────────────────────────────┐
    // │  Agent List          │  Network Graph (Canvas)        │
    // ├──────────────────────┴────────────────────────────────┤
    // │  Live Log + Stats                                      │
    // └───────────────────────────────────────────────────────┘

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(vert[0]);

    draw_agent_list(f, app, top[0]);
    graph::draw(f, app, top[1]);
    draw_log(f, app, vert[1]);
}

fn draw_agent_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.agents.iter().map(|a| {
        let priv_color = match a.privilege {
            oxide_common::types::Privilege::System => C_DANGER,
            oxide_common::types::Privilege::Admin  => C_WARN,
            oxide_common::types::Privilege::User   => C_OK,
        };
        let status_color = match a.status {
            oxide_common::types::AgentStatus::Active => C_OK,
            oxide_common::types::AgentStatus::Stale  => C_WARN,
            oxide_common::types::AgentStatus::Dead   => C_DANGER,
        };
        let line = Line::from(vec![
            Span::styled("● ", Style::default().fg(status_color)),
            Span::styled(&a.hostname, Style::default().fg(C_FG).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" [{}]", a.privilege), Style::default().fg(priv_color)),
        ]);
        ListItem::new(line)
    }).collect();

    let count = app.agents.len();
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(format!(" Agents ({count}) ")));
    f.render_widget(list, area);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let splits = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(area);

    // Log
    let log_items: Vec<ListItem> = app.log.iter().take(30).map(|msg| {
        let color = if msg.contains("ERROR") || msg.contains("FAIL") { C_DANGER }
                    else if msg.contains("NEW AGENT") { C_OK }
                    else { C_DIM };
        ListItem::new(Line::from(Span::styled(msg.as_str(), Style::default().fg(color))))
    }).collect();

    let log = List::new(log_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(" Activity Log "));
    f.render_widget(log, splits[0]);

    // Stats panel
    let active  = app.agents.iter().filter(|a| a.status == oxide_common::types::AgentStatus::Active).count();
    let stale   = app.agents.iter().filter(|a| a.status == oxide_common::types::AgentStatus::Stale).count();
    let systems = app.agents.iter().filter(|a| a.privilege == oxide_common::types::Privilege::System).count();
    let listeners = app.listeners.iter().filter(|l| l.status == oxide_common::types::ListenerStatus::Running).count();

    let stats = vec![
        Line::from(vec![Span::styled("  Agents:    ", Style::default().fg(C_DIM)),
            Span::styled(format!("{active}"), Style::default().fg(C_OK).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("  Stale:     ", Style::default().fg(C_DIM)),
            Span::styled(format!("{stale}"), Style::default().fg(C_WARN))]),
        Line::from(vec![Span::styled("  SYSTEM:    ", Style::default().fg(C_DIM)),
            Span::styled(format!("{systems}"), Style::default().fg(C_DANGER).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("  Listeners: ", Style::default().fg(C_DIM)),
            Span::styled(format!("{listeners}"), Style::default().fg(C_ACCENT))]),
        Line::from(vec![Span::styled("  Loot:      ", Style::default().fg(C_DIM)),
            Span::styled(format!("{}", app.loot.len()), Style::default().fg(C_ACCENT))]),
    ];

    let stat_para = Paragraph::new(stats)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(" Stats "));
    f.render_widget(stat_para, splits[1]);
}
