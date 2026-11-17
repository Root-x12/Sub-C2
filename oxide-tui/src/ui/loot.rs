use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_FG, C_WARN};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let header = Row::new(["Agent", "Kind", "Timestamp", "Data"])
        .style(Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = app.loot.iter().map(|e| {
        let preview = if e.data.len() > 60 {
            format!("{}…", &e.data[..60])
        } else {
            e.data.clone()
        };
        Row::new(vec![
            Cell::from(e.agent_id.to_string().chars().take(8).collect::<String>()).style(Style::default().fg(C_DIM)),
            Cell::from(e.kind.to_string()).style(Style::default().fg(C_WARN).add_modifier(Modifier::BOLD)),
            Cell::from(e.timestamp.format("%H:%M:%S").to_string()).style(Style::default().fg(C_DIM)),
            Cell::from(preview).style(Style::default().fg(C_FG)),
        ])
    }).collect();

    let count = app.loot.len();
    let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Min(20),
            ],
        )
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(format!(" Loot ({count}) ")));

    let mut state = TableState::default();
    state.select(app.selected_loot);
    f.render_stateful_widget(table, area, &mut state);
}
