mod listener;
mod payload;
mod interact;

use crate::app::{App, Modal};
use ratatui::{layout::Rect, Frame};

/// Draw a modal and return its bounding Rect for effect targeting.
pub fn draw(f: &mut Frame, app: &App, modal: &Modal, area: Rect) -> Rect {
    match modal {
        Modal::NewListener(draft)           => { listener::draw(f, draft, area); centered_rect(60, 55, area) },
        Modal::GenPayload(draft)            => { payload::draw(f, app, draft, area); centered_rect(80, 85, area) },
        Modal::AgentInteract { agent_id, input } => { interact::draw(f, app, *agent_id, input, area); centered_rect(70, 70, area) },
        Modal::Confirm { message, action }  => { draw_confirm(f, message, action, area); centered_rect(50, 20, area) },
    }
}

fn draw_confirm(f: &mut Frame, message: &str, action: &str, area: Rect) {
    use crate::ui::{C_ACCENT, C_SURFACE, C_DIM, C_FG, C_OK, C_DANGER};
    use ratatui::{
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph},
    };

    let popup = centered_rect(50, 20, area);
    f.render_widget(Clear, popup);

    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {message}"), Style::default().fg(C_FG))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[Y]", Style::default().fg(C_OK).add_modifier(Modifier::BOLD)),
            Span::styled(" Confirm  ", Style::default().fg(C_DIM)),
            Span::styled("[N]", Style::default().fg(C_DANGER).add_modifier(Modifier::BOLD)),
            Span::styled(" Cancel", Style::default().fg(C_DIM)),
        ]),
    ])
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(C_ACCENT))
        .style(Style::default().bg(C_SURFACE))
        .title(Span::styled(
            format!(" ◈ {action} "),
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        )));
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
