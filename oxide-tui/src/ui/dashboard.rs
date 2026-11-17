use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_DANGER, C_FG, C_LABEL, C_OK, C_SELECT, C_WARN, graph};
use oxide_common::types::{AgentStatus, ListenerStatus, Privilege};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let top_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
        .split(top_bottom[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(top_bottom[1]);

    draw_stats(f, app, top[0]);
    graph::draw(f, app, top[1]);
    draw_log(f, app, bottom[0]);
    draw_recent(f, app, bottom[1]);
}

fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let active   = app.agents.iter().filter(|a| a.status == AgentStatus::Active).count();
    let stale    = app.agents.iter().filter(|a| a.status == AgentStatus::Stale).count();
    let systems  = app.agents.iter().filter(|a| a.privilege == Privilege::System).count();
    let admins   = app.agents.iter().filter(|a| a.privilege == Privilege::Admin).count();
    let running  = app.listeners.iter().filter(|l| l.status == ListenerStatus::Running).count();

    let kv = |k: &'static str, v: String, color: Color| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<11} ", k), Style::default().fg(C_LABEL)),
            Span::styled(v, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ])
    };

    let lines = vec![
        Line::from(""),
        kv("Agents",    format!("{active}"),   C_OK),
        kv("Stale",     format!("{stale}"),    if stale > 0 { C_WARN } else { C_DIM }),
        kv("SYSTEM",    format!("{systems}"),  if systems > 0 { C_DANGER } else { C_DIM }),
        kv("Admin",     format!("{admins}"),   if admins > 0 { C_WARN } else { C_DIM }),
        Line::from(""),
        kv("Listeners", format!("{running}"),  C_ACCENT),
        kv("Loot",      format!("{}", app.loot.len()), C_ACCENT),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(Span::styled(
                " Stats ",
                Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
            )));
    f.render_widget(para, area);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.log.iter().take(50).map(|msg| {
        let color = if msg.contains("ERROR") || msg.contains("FAIL") || msg.contains("dead") {
            C_DANGER
        } else if msg.contains("NEW AGENT") || msg.contains("done") {
            C_OK
        } else if msg.contains("→") || msg.contains("←") {
            C_ACCENT
        } else {
            C_DIM
        };
        ListItem::new(Line::from(Span::styled(msg.as_str(), Style::default().fg(color))))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
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
            .title(Span::styled(" Agents ", Style::default().fg(C_ACCENT))));
    f.render_widget(para, area);
}
