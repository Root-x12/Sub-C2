use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_DANGER, C_FG, C_OK, C_SELECT, C_WARN};
use oxide_common::types::{AgentStatus, Privilege};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    draw_agent_list(f, app, panes[0]);
    draw_session_log(f, app, panes[1]);
}

fn draw_agent_list(f: &mut Frame, app: &mut App, area: Rect) {
    let now = chrono::Utc::now();
    let items: Vec<ListItem> = app.agents.iter().enumerate().map(|(i, a)| {
        let dot_color = match a.status {
            AgentStatus::Active => C_OK,
            AgentStatus::Stale  => C_WARN,
            AgentStatus::Dead   => C_DIM,
        };
        let dot_sym = match a.status {
            AgentStatus::Active => "● ",
            AgentStatus::Stale  => "◐ ",
            AgentStatus::Dead   => "○ ",
        };
        let (priv_tag, priv_color) = match a.privilege {
            Privilege::System => ("SYS", C_DANGER),
            Privilege::Admin  => ("ADM", C_WARN),
            Privilege::User   => ("USR", C_FG),
        };
        let elapsed_s = (now - a.last_seen).num_seconds().max(0);
        let age_str = if elapsed_s < 60 {
            format!("{elapsed_s}s")
        } else {
            format!("{}m", elapsed_s / 60)
        };
        Line::from(vec![
            Span::styled(format!(" {:>2} ", i + 1), Style::default().fg(C_DIM)),
            Span::styled(dot_sym, Style::default().fg(dot_color)),
            Span::styled(
                format!("{:<12}", &a.hostname),
                Style::default().fg(C_FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled(priv_tag, Style::default().fg(priv_color)),
            Span::styled(format!(" {age_str}"), Style::default().fg(C_DIM)),
        ])
    }).map(ListItem::new).collect();

    let count = app.agents.len();
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(Span::styled(
                format!(" Agents ({count}) "),
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(90, 20, 20))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(app.selected_agent);
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_session_log(f: &mut Frame, app: &App, area: Rect) {
    let now = chrono::Utc::now();
    let splits = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let (title, lines) = if let Some(idx) = app.selected_agent {
        if let Some(agent) = app.agents.get(idx) {
            let hist = app.agent_history.get(&agent.id);
            let elapsed_s = (now - agent.last_seen).num_seconds().max(0);
            let status_tag = match agent.status {
                AgentStatus::Active => format!("ACTIVE  last seen {elapsed_s}s ago"),
                AgentStatus::Stale  => format!("STALE   last seen {elapsed_s}s ago"),
                AgentStatus::Dead   => format!("DEAD    last seen {elapsed_s}s ago"),
            };
            let title = format!(" {} @ {}  [{}] ", agent.hostname, agent.ip, status_tag);

            let all_lines: Vec<&String> = hist.map(|h| h.iter().collect()).unwrap_or_default();
            let visible_h = splits[0].height.saturating_sub(2) as usize;
            let total = all_lines.len();
            let end = total;
            let start = end.saturating_sub(visible_h);
            let visible = &all_lines[start..end];

            let mut items: Vec<Line> = visible.iter().map(|msg| {
                let color = if msg.contains("→ ") {
                    C_ACCENT
                } else if msg.contains("← ") || msg.contains("● Connected") {
                    C_OK
                } else if msg.contains("[ERR]") {
                    C_DANGER
                } else if msg.contains("○ Agent lost") {
                    C_WARN
                } else {
                    C_FG
                };
                Line::from(Span::styled(*msg, Style::default().fg(color)))
            }).collect();

            // Warn user if agent looks dead but commands are still pending
            if matches!(agent.status, AgentStatus::Stale | AgentStatus::Dead) {
                items.push(Line::from(Span::styled(
                    format!("  ⚠ Agent {}: check if it is still running (last seen {elapsed_s}s ago)",
                        if matches!(agent.status, AgentStatus::Dead) { "DEAD" } else { "STALE" }),
                    Style::default().fg(C_WARN),
                )));
            }

            (title, items)
        } else {
            (" Session Log ".into(), vec![
                Line::from(Span::styled("  Select an agent with ↑↓", Style::default().fg(C_DIM)))
            ])
        }
    } else {
        (" Session Log ".into(), vec![
            Line::from(Span::styled("  Select an agent with ↑↓", Style::default().fg(C_DIM)))
        ])
    };

    let para = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(Span::styled(title, Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD))));
    f.render_widget(para, splits[0]);

    // Footer
    let footer = Line::from(vec![
        Span::styled(" [↑↓] select agent  ", Style::default().fg(C_DIM)),
        Span::styled("[Enter]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" open interact  ", Style::default().fg(C_DIM)),
    ]);
    f.render_widget(Paragraph::new(footer), splits[1]);
}
