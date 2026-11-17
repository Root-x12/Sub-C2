//! Generate Payload modal — the centrepiece of OXIDE's TUI.
//! Features a live OPSEC Score bar that updates as options are toggled.

use crate::app::{App, PayloadDraft};
use crate::ui::modals::centered_rect;
use crate::ui::{C_ACCENT, C_BORDER, C_DANGER, C_DIM, C_FG, C_OK, C_WARN};
use oxide_common::types::BuildOptions;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, draft: &PayloadDraft, area: Rect) {
    let popup = centered_rect(80, 85, area);
    f.render_widget(Clear, popup);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),     // main content
            Constraint::Length(5),  // opsec score bar
            Constraint::Length(3),  // footer buttons
        ])
        .margin(1)
        .split(popup);

    // Outer border
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_ACCENT))
            .title(Span::styled(
                " ◈ Generate Payload — OXIDE C2 ◈ ",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )),
        popup,
    );

    // ── Header subtitle
    let subtitle = Paragraph::new(Line::from(vec![
        Span::styled("  Configure payload evasion profile. ", Style::default().fg(C_DIM)),
        Span::styled("[Tab/↑↓]", Style::default().fg(C_ACCENT)),
        Span::styled(" navigate  ", Style::default().fg(C_DIM)),
        Span::styled("[←→]", Style::default().fg(C_ACCENT)),
        Span::styled(" cycle  ", Style::default().fg(C_DIM)),
        Span::styled("[Space]", Style::default().fg(C_ACCENT)),
        Span::styled(" toggle  ", Style::default().fg(C_DIM)),
        Span::styled("[Enter]", Style::default().fg(C_OK)),
        Span::styled(" build", Style::default().fg(C_DIM)),
    ]));
    f.render_widget(subtitle, root[0]);

    // ── Main 2-column layout
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[1]);

    draw_left(f, app, draft, cols[0]);
    draw_right(f, draft, cols[1]);

    // ── OPSEC Score bar
    draw_opsec_bar(f, draft, app, root[2]);

    // ── Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("[ Enter: BUILD ]", Style::default().fg(C_OK).add_modifier(Modifier::BOLD)),
        Span::styled("   ", Style::default()),
        Span::styled("[ Esc: cancel ]", Style::default().fg(C_DANGER)),
    ]));
    f.render_widget(footer, root[3]);
}

fn field_style(focused: usize, field: usize) -> Style {
    if focused == field {
        Style::default().fg(Color::Black).bg(C_ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(C_FG)
    }
}

fn toggle_str(on: bool) -> Span<'static> {
    if on {
        Span::styled("[✓]", Style::default().fg(C_OK).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[ ]", Style::default().fg(C_DIM))
    }
}

fn draw_left(f: &mut Frame, app: &App, draft: &PayloadDraft, area: Rect) {
    let listener_name = app.listeners
        .get(draft.listener_idx)
        .map(|l| format!("{} ({}:{})", l.config.name, l.config.host, l.config.port))
        .unwrap_or_else(|| "(no listeners)".into());

    let format_names  = PayloadDraft::formats();
    let sleep_names   = PayloadDraft::sleep_obfs();
    let inject_names  = PayloadDraft::injections();

    let lines = vec![
        Line::from(Span::styled("  ── Payload Config ──────────────────", Style::default().fg(C_DIM))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Listener: ", Style::default().fg(C_DIM)),
            Span::styled(format!("◄ {listener_name} ►"), field_style(draft.focused, 0)),
        ]),
        Line::from(vec![
            Span::styled("  Format:   ", Style::default().fg(C_DIM)),
            Span::styled(format!("◄ {} ►", format_names[draft.format_idx]), field_style(draft.focused, 1)),
        ]),
        Line::from(vec![
            Span::styled("  Sleep ms: ", Style::default().fg(C_DIM)),
            Span::styled(format!(" {} ", draft.sleep_ms), field_style(draft.focused, 2)),
        ]),
        Line::from(vec![
            Span::styled("  Jitter %: ", Style::default().fg(C_DIM)),
            Span::styled(format!(" {} ", draft.jitter_pct), field_style(draft.focused, 3)),
        ]),
        Line::from(vec![
            Span::styled("  Output:   ", Style::default().fg(C_DIM)),
            Span::styled(format!(" {} ", draft.output_path), field_style(draft.focused, 4)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  ── Obfuscation ─────────────────────", Style::default().fg(C_DIM))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Sleep Obf:  ", Style::default().fg(C_DIM)),
            Span::styled(format!("◄ {} ►", sleep_names[draft.sleep_obf_idx]), field_style(draft.focused, 5)),
        ]),
        Line::from(vec![
            Span::styled("  Injection:  ", Style::default().fg(C_DIM)),
            Span::styled(format!("◄ {} ►", inject_names[draft.injection_idx]), field_style(draft.focused, 6)),
        ]),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(para, area);
}

fn draw_right(f: &mut Frame, draft: &PayloadDraft, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("  ── Detection Evasion ───────────────", Style::default().fg(C_DIM))),
        Line::from(""),
        bool_row(draft.focused,  7, "AMSI Bypass",        draft.amsi),
        bool_row(draft.focused,  8, "ETW Bypass",         draft.etw),
        bool_row(draft.focused,  9, "Sandbox Evasion",    draft.sandbox),
        bool_row(draft.focused, 10, "Indirect Syscalls",  draft.syscalls),
        bool_row(draft.focused, 11, "Stack Spoof",        draft.stack_spoof),
        bool_row(draft.focused, 12, "Module Stomp",       draft.module_stomp),
        bool_row(draft.focused, 13, "Block non-MS DLLs",  draft.block_dlls),
        bool_row(draft.focused, 14, "Encrypted Comms",    draft.encrypted),
        bool_row(draft.focused, 15, "DNS DGA",            draft.dns_dga),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(para, area);
}

fn bool_row<'a>(focused: usize, field: usize, label: &'a str, on: bool) -> Line<'a> {
    let focus_indicator = if focused == field {
        Span::styled("▶ ", Style::default().fg(C_ACCENT))
    } else {
        Span::raw("  ")
    };
    Line::from(vec![
        focus_indicator,
        toggle_str(on),
        Span::styled(format!(" {label}"), Style::default().fg(C_FG)),
    ])
}

fn draw_opsec_bar(f: &mut Frame, draft: &PayloadDraft, app: &App, area: Rect) {
    // Calculate score from current options
    let opts = draft.to_build_options(&app.listeners);
    let score = opts.opsec_score();
    let (label, _color_name) = BuildOptions::opsec_label(score);

    let bar_color = if score >= 75 { C_OK }
                    else if score >= 45 { C_WARN }
                    else { C_DANGER };

    let title = format!(" OPSEC Score: {score}% — {label} ");

    let gauge = Gauge::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(bar_color))
            .title(Span::styled(
                title,
                Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
            )))
        .gauge_style(Style::default().fg(bar_color).bg(Color::Black))
        .percent(score as u16)
        .label(Span::styled(
            format!("{score}%"),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(gauge, area);
}
