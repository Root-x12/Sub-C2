use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_DANGER, C_FG, C_LABEL, C_OK, C_SELECT, C_WARN};
use oxide_common::types::ListenerStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_list(f, app, panes[0]);
    draw_detail(f, app, panes[1]);
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let filter = app.filter.to_lowercase();
    let filtered: Vec<(usize, &oxide_common::types::ListenerInfo)> = app.listeners
        .iter()
        .enumerate()
        .filter(|(_, l)| filter.is_empty() || l.config.name.to_lowercase().contains(&filter))
        .collect();

    let items: Vec<ListItem> = filtered.iter().map(|(real_idx, l)| {
        let (dot, dot_color) = match &l.status {
            ListenerStatus::Running  => ("◉", C_OK),
            ListenerStatus::Stopped  => ("○", C_DIM),
            ListenerStatus::Error(_) => ("✕", C_DANGER),
        };
        let num = real_idx + 1;
        let proto = l.config.protocol.to_string().to_uppercase();
        Line::from(vec![
            Span::styled(format!(" {:>2} ", num), Style::default().fg(C_DIM)),
            Span::styled(format!("{dot} "), Style::default().fg(dot_color)),
            Span::styled(
                format!("{:<16}", &l.config.name),
                Style::default().fg(C_FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{:<6}", proto), Style::default().fg(C_ACCENT)),
            Span::styled(format!(":{}", l.config.port), Style::default().fg(C_DIM)),
        ])
    }).map(ListItem::new).collect();

    let filtered_sel = app.selected_listener.and_then(|sel| {
        filtered.iter().position(|(ri, _)| *ri == sel)
    });

    let filter_hint = if !app.filter.is_empty() {
        format!("  /{} ", app.filter)
    } else {
        String::new()
    };
    let title = format!(" Listeners ({}) {} ", filtered.len(), filter_hint);

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(Span::styled(title, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD))))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(90, 20, 20))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(filtered_sel);
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let listener = app.selected_listener.and_then(|i| app.listeners.get(i));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(C_BORDER))
        .title(Span::styled(
            listener.map(|l| format!(" {} ", l.config.name)).unwrap_or_else(|| " — ".into()),
            Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(l) = listener {
        let (status_str, status_color) = match &l.status {
            ListenerStatus::Running  => ("RUNNING", C_OK),
            ListenerStatus::Stopped  => ("STOPPED", C_DIM),
            ListenerStatus::Error(_) => ("ERROR",   C_DANGER),
        };
        let error_msg = match &l.status {
            ListenerStatus::Error(e) => Some(e.clone()),
            _ => None,
        };

        let kv = |k: &'static str, v: String| -> Line<'static> {
            Line::from(vec![
                Span::styled(format!("  {:<13}: ", k), Style::default().fg(C_LABEL)),
                Span::styled(v, Style::default().fg(C_FG)),
            ])
        };

        let mut lines = vec![
            Line::from(""),
            kv("Name",     l.config.name.clone()),
            kv("Protocol", l.config.protocol.to_string().to_uppercase()),
            kv("Host",     l.config.host.clone()),
            kv("Port",     l.config.port.to_string()),
            Line::from(vec![
                Span::styled(format!("  {:<13}: ", "Status"), Style::default().fg(C_LABEL)),
                Span::styled(status_str, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            ]),
            kv("Active Agents", l.agents.to_string()),
        ];

        if let Some(e) = error_msg {
            lines.push(Line::from(vec![
                Span::styled("  Error       : ", Style::default().fg(C_LABEL)),
                Span::styled(e, Style::default().fg(C_DANGER)),
            ]));
        }

        if let Some(ref domain) = l.config.dns_domain {
            lines.push(kv("DNS Domain", domain.clone()));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ─────────────────────────────────────────────",
            Style::default().fg(C_DIM),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  [N]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" new listener   ", Style::default().fg(C_DIM)),
            Span::styled("[S]", Style::default().fg(C_WARN).add_modifier(Modifier::BOLD)),
            Span::styled(" stop selected   ", Style::default().fg(C_DIM)),
            Span::styled("[↑↓]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" select", Style::default().fg(C_DIM)),
        ]));

        let para = Paragraph::new(lines);
        f.render_widget(para, inner);
    } else {
        let hint = Paragraph::new(Line::from(Span::styled(
            "\n  No listeners. Press [N] to create one.",
            Style::default().fg(C_DIM),
        )));
        f.render_widget(hint, inner);
    }
}
