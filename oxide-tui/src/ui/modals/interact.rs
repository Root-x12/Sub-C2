use crate::app::App;
use crate::ui::modals::centered_rect;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_FG};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use uuid::Uuid;

pub fn draw(f: &mut Frame, app: &App, agent_id: Uuid, input: &str, area: Rect) {
    let popup = centered_rect(80, 70, area);
    f.render_widget(Clear, popup);

    let agent = app.agents.iter().find(|a| a.id == agent_id);
    let title = agent.map(|a| format!(" ◈ {} @ {} ◈ ", a.hostname, a.ip))
        .unwrap_or_else(|| " ◈ Agent Interact ◈ ".into());

    // Split: output log + input line
    let splits = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .margin(1)
        .split(popup);

    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_ACCENT))
            .title(Span::styled(title, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD))),
        popup,
    );

    // Log entries relevant to this agent
    let log_items: Vec<ListItem> = app.log.iter()
        .take(50)
        .map(|msg| ListItem::new(Line::from(Span::styled(msg.as_str(), Style::default().fg(C_FG)))))
        .collect();

    let log_list = List::new(log_items)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(log_list, splits[0]);

    // Command input
    let input_line = Paragraph::new(Line::from(vec![
        Span::styled("❯ ", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(input, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(C_ACCENT)), // cursor
    ]))
    .block(Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(C_DIM))
        .title(Span::styled(
            " command [Enter: send, Esc: close] ",
            Style::default().fg(C_DIM),
        )));
    f.render_widget(input_line, splits[1]);
}
