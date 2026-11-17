use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_BG, C_DIM, C_DANGER, C_FG, C_OK, C_SELECT, C_WARN, graph};
use oxide_common::types::{AgentStatus, ListenerStatus, Privilege};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Sparkline},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let top_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
        .split(top_bottom[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(top_bottom[1]);

    // Bottom-right: sparkline strip (5 rows) above recent-agents list
    let br = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(bottom[1]);

    draw_stats(f, app, top[0]);
    graph::draw(f, app, top[1]);
    draw_log(f, app, bottom[0]);
    draw_sparkline(f, app, br[0]);
    draw_recent(f, app, br[1]);
}

fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let active   = app.agents.iter().filter(|a| a.status == AgentStatus::Active).count();
    let stale    = app.agents.iter().filter(|a| a.status == AgentStatus::Stale).count();
    let systems  = app.agents.iter().filter(|a| a.privilege == Privilege::System).count();
    let admins   = app.agents.iter().filter(|a| a.privilege == Privilege::Admin).count();
    let running  = app.listeners.iter().filter(|l| l.status == ListenerStatus::Running).count();

    let stat = |label: &'static str, val: usize, color: Color| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<11}", label), Style::default().fg(C_DIM)),
            Span::styled(format!("{:>4}", val),
                Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ])
    };

    let sep = Line::from(Span::styled(
        "  ──────────────────",
        Style::default().fg(C_DIM),
    ));

    let lines = vec![
        Line::from(""),
        stat("AGENTS",    active,          C_OK),
        stat("Stale",     stale,           if stale   > 0 { C_WARN }   else { C_DIM }),
        stat("SYSTEM",    systems,         if systems > 0 { C_DANGER } else { C_DIM }),
        stat("Admin",     admins,          if admins  > 0 { C_WARN }   else { C_DIM }),
        Line::from(""),
        sep,
        Line::from(""),
        stat("Listeners", running,         C_ACCENT),
        stat("Loot",      app.loot.len(), C_ACCENT),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG))
            .title(Span::styled(
                " Stats ",
                Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
            )));
    f.render_widget(para, area);
}

fn draw_sparkline(f: &mut Frame, app: &App, area: Rect) {
    let data: Vec<u64> = app.beacon_history.iter().copied().collect();

    let sparkline = Sparkline::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG))
            .title(Span::styled(" Beacons/s ", Style::default().fg(C_ACCENT))))
        .data(&data)
        .style(Style::default().fg(C_ACCENT));
    f.render_widget(sparkline, area);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.log.iter().take(200).map(|msg| {
        // Parse "[HH:MM:SS] body"
        let (ts, body) = if msg.starts_with('[') {
            msg.split_once("] ")
                .map(|(t, r)| (format!("{t}]"), r.to_string()))
                .unwrap_or_else(|| (String::new(), msg.clone()))
        } else {
            (String::new(), msg.clone())
        };

        let body_color = if body.contains("NEW AGENT") || body.starts_with("build done") {
            C_OK
        } else if body.contains("dead") || body.contains("FAILED") || body.contains("ERROR") {
            C_DANGER
        } else if body.starts_with("←") || body.starts_with("  ") {
            C_FG
        } else if body.starts_with("→") {
            C_ACCENT
        } else {
            C_DIM
        };

        ListItem::new(Line::from(vec![
            Span::styled(ts, Style::default().fg(C_DIM)),
            Span::styled("  ", Style::default()),
            Span::styled(body, Style::default().fg(body_color)),
        ]))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG))
            .title(Span::styled(" Activity Log ", Style::default().fg(C_ACCENT))));
    f.render_widget(list, area);
}

fn draw_recent(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<Line> = app.agents.iter().map(|a| {
        let dot_color = match a.status {
            AgentStatus::Active => C_OK,
            AgentStatus::Stale  => C_WARN,
            AgentStatus::Dead   => C_DIM,
        };
        let (priv_str, priv_color) = match a.privilege {
            Privilege::System => ("SYS", C_DANGER),
            Privilege::Admin  => ("ADM", C_WARN),
            Privilege::User   => ("USR", C_FG),
        };
        Line::from(vec![
            Span::styled("● ", Style::default().fg(dot_color)),
            Span::styled(
                format!("{:<12}", &a.hostname),
                Style::default().fg(C_FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled(priv_str, Style::default().fg(priv_color)),
        ])
    }).collect();

    let para = Paragraph::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG))
            .title(Span::styled(" Agents ", Style::default().fg(C_ACCENT))));
    f.render_widget(para, area);
}
