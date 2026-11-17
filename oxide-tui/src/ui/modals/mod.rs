mod listener;
mod payload;
mod interact;

use crate::app::{App, Modal};
use ratatui::{layout::Rect, Frame};

pub fn draw(f: &mut Frame, app: &App, modal: &Modal, area: Rect) {
    match modal {
        Modal::NewListener(draft)           => listener::draw(f, draft, area),
        Modal::GenPayload(draft)            => payload::draw(f, app, draft, area),
        Modal::AgentInteract { agent_id, input } => interact::draw(f, app, *agent_id, input, area),
        Modal::Confirm { message, action }  => draw_confirm(f, message, action, area),
    }
}

fn draw_confirm(f: &mut Frame, message: &str, action: &str, area: Rect) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph},
    };

    let popup = centered_rect(50, 20, area);
    f.render_widget(Clear, popup);

    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" Confirm  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[N]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(" {} ", action)));
    f.render_widget(para, popup);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
