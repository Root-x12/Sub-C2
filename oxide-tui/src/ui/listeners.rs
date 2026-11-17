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

    let header = Row::new(["ID", "Name", "Protocol", "Host", "Port", "Agents", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD))))
        .height(1);

    let rows: Vec<Row> = app.listeners.iter().map(|l| {
        let status_color = match &l.status {
            oxide_common::types::ListenerStatus::Running => C_OK,
            oxide_common::types::ListenerStatus::Stopped => C_WARN,
            oxide_common::types::ListenerStatus::Error(_) => C_DANGER,
        };
        Row::new(vec![
            Cell::from(l.config.id.to_string().chars().take(8).collect::<String>()).style(Style::default().fg(C_DIM)),
            Cell::from(l.config.name.as_str()).style(Style::default().fg(C_FG).add_modifier(Modifier::BOLD)),
            Cell::from(l.config.protocol.to_string()).style(Style::default().fg(C_ACCENT)),
            Cell::from(l.config.host.as_str()).style(Style::default().fg(C_DIM)),
            Cell::from(l.config.port.to_string()).style(Style::default().fg(C_FG)),
            Cell::from(l.agents.to_string()).style(Style::default().fg(C_OK)),
            Cell::from(l.status.to_string()).style(Style::default().fg(status_color)),
        ])
    }).collect();

    let count = app.listeners.len();
    let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Percentage(20),
                Constraint::Length(10),
                Constraint::Percentage(25),
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Min(10),
            ],
        )
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(format!(" Listeners ({count}) — [N] new listener ")));

    let mut state = TableState::default();
    state.select(app.selected_listener);
    f.render_stateful_widget(table, chunks[0], &mut state);

    let help = Line::from(vec![
        Span::styled(" [N] new listener  ", Style::default().fg(C_ACCENT)),
        Span::styled("[S] stop selected  ", Style::default().fg(C_DIM)),
        Span::styled("[Tab] switch tab", Style::default().fg(C_DIM)),
    ]);
    f.render_widget(ratatui::widgets::Paragraph::new(help), chunks[1]);
}
