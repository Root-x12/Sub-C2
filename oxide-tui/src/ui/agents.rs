use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_DANGER, C_FG, C_OK, C_WARN};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(area);

    let header_cells = ["ID", "Hostname", "IP", "User", "OS", "Privilege", "PID", "Sleep", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app.agents.iter().map(|a| {
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
        Row::new(vec![
            Cell::from(a.id.to_string().chars().take(8).collect::<String>()).style(Style::default().fg(C_DIM)),
            Cell::from(a.hostname.as_str()).style(Style::default().fg(C_FG).add_modifier(Modifier::BOLD)),
            Cell::from(a.ip.as_str()).style(Style::default().fg(C_DIM)),
            Cell::from(a.username.as_str()).style(Style::default().fg(C_FG)),
            Cell::from(a.os.as_str()).style(Style::default().fg(C_DIM)),
            Cell::from(a.privilege.to_string()).style(Style::default().fg(priv_color).add_modifier(Modifier::BOLD)),
            Cell::from(a.pid.to_string()).style(Style::default().fg(C_DIM)),
            Cell::from(format!("{}ms", a.sleep_ms)).style(Style::default().fg(C_DIM)),
            Cell::from(a.status.to_string()).style(Style::default().fg(status_color)),
        ])
    }).collect();

    let count = app.agents.len();
    let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Percentage(20),
                Constraint::Length(16),
                Constraint::Percentage(15),
                Constraint::Percentage(10),
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(8),
            ],
        )
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(format!(" Agents ({count}) — ↑↓ select, Enter interact ")));

    let mut state = TableState::default();
    state.select(app.selected_agent);
    f.render_stateful_widget(table, chunks[0], &mut state);

    // Status bar
    let help = Line::from(vec![
        Span::styled(" [↑↓] select  ", Style::default().fg(C_DIM)),
        Span::styled("[Enter] interact  ", Style::default().fg(C_ACCENT)),
        Span::styled("[Tab] switch tab  ", Style::default().fg(C_DIM)),
    ]);
    f.render_widget(ratatui::widgets::Paragraph::new(help), chunks[1]);
}
