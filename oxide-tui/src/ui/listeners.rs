use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_DANGER, C_FG, C_LABEL, C_OK, C_SELECT, C_WARN};
use oxide_common::types::ListenerStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(area);

    draw_list(f, app, panes[0]);
    draw_detail(f, app, panes[1]);
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let splits = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // ── Search bar ────────────────────────────────────────────────────────────
    let search_text = if app.filter_active {
        format!("{}_", app.filter)
    } else {
        app.filter.clone()
    };
    let search_bar = Paragraph::new(Line::from(vec![
        Span::styled("Search: ", Style::default().fg(C_DIM)),
        Span::styled(search_text, Style::default().fg(C_FG)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(C_BORDER)));
    f.render_widget(search_bar, splits[0]);

    // ── List ─────────────────────────────────────────────────────────────────
    let filter = app.filter.to_lowercase();
    let filtered: Vec<(usize, &oxide_common::types::ListenerInfo)> = app.listeners
        .iter()
        .enumerate()
        .filter(|(_, l)| filter.is_empty() || l.config.name.to_lowercase().contains(&filter))
        .collect();

    let items: Vec<ListItem> = filtered.iter().map(|(real_idx, l)| {
        let (dot, _dot_color) = match &l.status {
            ListenerStatus::Running  => ("◉", C_OK),
            ListenerStatus::Stopped  => ("○", C_DIM),
            ListenerStatus::Error(_) => ("✕", C_DANGER),
        };
        let num = real_idx + 1;
        let proto = l.config.protocol.to_string().to_uppercase();
        Line::from(vec![
            Span::styled(format!("  {:>2}  {dot}  ", num), Style::default().fg(C_DIM)),
            Span::styled(format!("{:<18}", &l.config.name), Style::default().fg(C_FG)),
            Span::styled(format!("{:<6}", proto), Style::default().fg(C_DIM)),
        ])
    }).map(ListItem::new).collect();

    let filtered_sel = app.selected_listener.and_then(|sel| {
        filtered.iter().position(|(ri, _)| *ri == sel)
    });

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(
            Style::default()
                .fg(C_SELECT)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(filtered_sel);
    f.render_stateful_widget(list, splits[1], &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let listener = app.selected_listener.and_then(|i| app.listeners.get(i));

    if let Some(l) = listener {
        let (status_str, status_color) = match &l.status {
            ListenerStatus::Running  => ("RUNNING", C_OK),
            ListenerStatus::Stopped  => ("STOPPED", C_DIM),
            ListenerStatus::Error(_) => ("ERROR",   C_DANGER),
        };
        let error_line = match &l.status {
            ListenerStatus::Error(e) => Some(e.clone()),
            _ => None,
        };

        let kv = |k: &'static str, v: String| -> Line<'static> {
            Line::from(vec![
                Span::styled(format!("  {:<14}", k), Style::default().fg(C_LABEL).add_modifier(Modifier::BOLD)),
                Span::styled(" :  ", Style::default().fg(C_DIM)),
                Span::styled(v, Style::default().fg(C_FG)),
            ])
        };

        let sep = Span::styled(
            "  ────────────────────────────────────────────────────────",
            Style::default().fg(C_DIM),
        );

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&l.config.name, Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("  {}:{}", l.config.host, l.config.port),
                    Style::default().fg(C_DIM),
                ),
            ]),
            Line::from(""),
            Line::from(sep.clone()),
            Line::from(""),
            kv("Name",          l.config.name.clone()),
            kv("Protocol",      l.config.protocol.to_string().to_uppercase()),
            kv("Host",          l.config.host.clone()),
            kv("Port",          l.config.port.to_string()),
            Line::from(vec![
                Span::styled(format!("  {:<14}", "Status"),
                    Style::default().fg(C_LABEL).add_modifier(Modifier::BOLD)),
                Span::styled(" :  ", Style::default().fg(C_DIM)),
                Span::styled(status_str, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            ]),
            kv("Active Agents",  l.agents.to_string()),
        ];

        if let Some(e) = error_line {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<14}", "Error"),
                    Style::default().fg(C_LABEL).add_modifier(Modifier::BOLD)),
                Span::styled(" :  ", Style::default().fg(C_DIM)),
                Span::styled(e, Style::default().fg(C_DANGER)),
            ]));
        }

        if let Some(ref domain) = l.config.dns_domain {
            lines.push(kv("DNS Domain", domain.clone()));
        }

        lines.extend([
            Line::from(""),
            Line::from(sep.clone()),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[N]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" new   ", Style::default().fg(C_DIM)),
                Span::styled("[S]", Style::default().fg(C_WARN).add_modifier(Modifier::BOLD)),
                Span::styled(" stop selected   ", Style::default().fg(C_DIM)),
                Span::styled("[↑↓]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" select", Style::default().fg(C_DIM)),
            ]),
        ]);

        f.render_widget(Paragraph::new(lines), area);
    } else {
        let lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled("  No listeners.", Style::default().fg(C_DIM))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Press ", Style::default().fg(C_DIM)),
                Span::styled("[N]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" to create one.", Style::default().fg(C_DIM)),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), area);
    }
}
