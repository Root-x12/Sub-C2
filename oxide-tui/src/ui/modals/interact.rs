use crate::app::App;
use crate::ui::modals::centered_rect;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_DANGER, C_FG, C_OK, C_SELECT, C_WARN};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use uuid::Uuid;

pub fn draw(f: &mut Frame, app: &App, agent_id: Uuid, input: &str, area: Rect) {
    let popup = centered_rect(85, 75, area);
    f.render_widget(Clear, popup);

    let agent = app.agents.iter().find(|a| a.id == agent_id);
    let title = agent
        .map(|a| format!(" ◈ {} @ {} ◈ ", a.hostname, a.ip))
        .unwrap_or_else(|| " ◈ Agent Interact ◈ ".into());

    // Outer border
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_SELECT))
            .title(Span::styled(title, Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD))),
        popup,
    );

    // Inner layout: history (scrollable) + input + hint
    let splits = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // history
            Constraint::Length(3),  // input bar
            Constraint::Length(1),  // key hints
        ])
        .margin(1)
        .split(popup);

    // ── History output ────────────────────────────────────────────────────────
    let hist_height = splits[0].height as usize;
    let all_lines: Vec<&String> = app.agent_history
        .get(&agent_id)
        .map(|h| h.iter().collect())
        .unwrap_or_default();

    let total = all_lines.len();
    let scroll = app.interact_scroll.min(total.saturating_sub(1));
    let end = total.saturating_sub(scroll);
    let start = end.saturating_sub(hist_height);
    let visible = &all_lines[start..end];

    let hist_lines: Vec<Line> = visible.iter().map(|msg| {
        let color = if msg.contains("→ ") {
            C_ACCENT
        } else if msg.contains("← ") || msg.contains("● Connected") {
            C_OK
        } else if msg.contains("[ERR]") || msg.contains("○ Agent") {
            C_DANGER
        } else {
            C_FG
        };
        Line::from(Span::styled(*msg, Style::default().fg(color)))
    }).collect();

    // Scroll indicator in top-right of history area
    let scroll_label = if scroll > 0 {
        format!(" ↑ scroll ({scroll}) ")
    } else {
        String::new()
    };

    let hist_block = Block::default()
        .borders(Borders::NONE)
        .title(Span::styled(scroll_label, Style::default().fg(C_WARN)));

    let hist_para = Paragraph::new(hist_lines)
        .block(hist_block);
    f.render_widget(hist_para, splits[0]);

    // ── Command input ─────────────────────────────────────────────────────────
    let input_line = Paragraph::new(Line::from(vec![
        Span::styled("❯ ", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(input, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(C_ACCENT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(C_BORDER))
            .title(Span::styled(" command ", Style::default().fg(C_DIM))),
    );
    f.render_widget(input_line, splits[1]);

    // ── Key hints ─────────────────────────────────────────────────────────────
    let hist_count = app.cmd_history.len();
    let hist_hint = if hist_count > 0 {
        format!("  [↑↓] history ({hist_count})  ")
    } else {
        "  ".into()
    };
    let hints = Line::from(vec![
        Span::styled(" [Enter]", Style::default().fg(C_ACCENT)),
        Span::styled(" send", Style::default().fg(C_DIM)),
        Span::styled(hist_hint, Style::default().fg(C_DIM)),
        Span::styled("[PgUp/Dn]", Style::default().fg(C_ACCENT)),
        Span::styled(" scroll  ", Style::default().fg(C_DIM)),
        Span::styled("[Esc]", Style::default().fg(C_DIM)),
        Span::styled(" close", Style::default().fg(C_DIM)),
    ]);
    f.render_widget(Paragraph::new(hints), splits[2]);
}
