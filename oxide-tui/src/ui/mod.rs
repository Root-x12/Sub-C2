mod agents;
mod dashboard;
mod graph;
mod listeners;
mod loot;
mod modals;
mod sessions;

use crate::app::{App, Modal, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};

// ── Colour palette ────────────────────────────────────────────────────────────
pub const C_ACCENT:  Color = Color::Cyan;
pub const C_OK:      Color = Color::Green;
pub const C_WARN:    Color = Color::Yellow;
pub const C_DANGER:  Color = Color::Red;
pub const C_DIM:     Color = Color::DarkGray;
pub const C_FG:      Color = Color::White;
pub const C_BG:      Color = Color::Black;
pub const C_BORDER:  Color = Color::DarkGray;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // ── Root split: tab bar (3) + content (rest)
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    draw_tabs(f, app, root[0]);

    match app.tab {
        Tab::Dashboard => dashboard::draw(f, app, root[1]),
        Tab::Agents    => agents::draw(f, app, root[1]),
        Tab::Listeners => listeners::draw(f, app, root[1]),
        Tab::Sessions  => sessions::draw(f, app, root[1]),
        Tab::Loot      => loot::draw(f, app, root[1]),
        Tab::Build     => draw_build(f, app, root[1]),
    }

    // Overlay modals last
    if let Some(modal) = &app.modal.clone() {
        modals::draw(f, app, modal, area);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == app.tab {
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(C_DIM)
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_BORDER))
                .title(Span::styled(
                    " ◈ OXIDE C2 — Adversary Emulation Framework ◈ ",
                    Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                )),
        )
        .select(app.tab.index())
        .highlight_style(Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD));

    f.render_widget(tabs, area);
}

fn draw_build(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::{
        text::Text,
        widgets::Paragraph,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .margin(1)
        .split(area);

    let content = vec![
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(C_DIM)),
            Span::styled("[G]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" to open the Payload Generator", Style::default().fg(C_FG)),
        ]),
        Line::from(""),
    ];

    // Build status
    if let Some(ref status) = app.build_status {
        let color = if status.starts_with("FAILED") { C_DANGER }
                    else if status.starts_with("Done") { C_OK }
                    else { C_WARN };

        let para = Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled("  Status: ", Style::default().fg(C_DIM)),
                Span::styled(status.as_str(), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ]),
        ]))
        .block(Block::default().borders(Borders::ALL)
            .title(" Build Status ")
            .border_style(Style::default().fg(C_BORDER)));
        f.render_widget(para, chunks[1]);
    }

    let para = Paragraph::new(Text::from(content))
        .block(Block::default().borders(Borders::ALL)
            .title(" Payload Generator ")
            .border_style(Style::default().fg(C_BORDER)));
    f.render_widget(para, chunks[0]);
}
