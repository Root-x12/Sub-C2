use crate::app::ListenerDraft;
use crate::ui::modals::centered_rect;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_FG};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, draft: &ListenerDraft, area: Rect) {
    let popup = centered_rect(60, 55, area);
    f.render_widget(Clear, popup);

    fn field<'a>(focused: usize, field: usize, label: &'a str, value: &'a str) -> Line<'a> {
        let val_style = if focused == field {
            Style::default().fg(Color::Black).bg(C_ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_FG)
        };
        let focus_mark = if focused == field {
            Span::styled("▶ ", Style::default().fg(C_ACCENT))
        } else {
            Span::raw("  ")
        };
        Line::from(vec![
            focus_mark,
            Span::styled(label, Style::default().fg(C_DIM)),
            Span::styled(format!(" {value} "), val_style),
        ])
    }

    let protocol = ListenerDraft::protocols()[draft.protocol];
    let protocol_line = {
        let focus_mark = if draft.focused == 1 {
            Span::styled("▶ ", Style::default().fg(C_ACCENT))
        } else {
            Span::raw("  ")
        };
        let val_style = if draft.focused == 1 {
            Style::default().fg(Color::Black).bg(C_ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_FG)
        };
        Line::from(vec![
            focus_mark,
            Span::styled("Protocol: ", Style::default().fg(C_DIM)),
            Span::styled(format!("◄ {protocol} ►"), val_style),
        ])
    };

    let lines = vec![
        Line::from(""),
        field(draft.focused, 0, "Name:     ", &draft.name),
        Line::from(""),
        protocol_line,
        Line::from(""),
        field(draft.focused, 2, "Host:     ", &draft.host),
        field(draft.focused, 3, "Port:     ", &draft.port),
        field(draft.focused, 4, "Domain:   ", &draft.domain),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[ Enter: CREATE ]", Style::default().fg(crate::ui::C_OK).add_modifier(Modifier::BOLD)),
            Span::styled("   ", Style::default()),
            Span::styled("[ Esc: cancel ]", Style::default().fg(crate::ui::C_DANGER)),
        ]),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_ACCENT))
            .title(Span::styled(
                " ◈ New Listener ◈ ",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )));
    f.render_widget(para, popup);
}
