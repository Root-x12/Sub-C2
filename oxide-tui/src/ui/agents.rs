use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_DANGER, C_FG, C_LABEL, C_OK, C_SELECT, C_WARN};
use oxide_common::types::{AgentStatus, Privilege};
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
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    draw_list(f, app, panes[0]);
    draw_detail(f, app, panes[1]);
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let filter = app.filter.to_lowercase();
    let filtered: Vec<(usize, &oxide_common::types::AgentInfo)> = app.agents
        .iter()
        .enumerate()
        .filter(|(_, a)| filter.is_empty() || a.hostname.to_lowercase().contains(&filter))
        .collect();

    // Build list items — numbered, with status dot and privilege
    let items: Vec<ListItem> = filtered.iter().map(|(real_idx, a)| {
        let dot_color = match a.status {
            AgentStatus::Active => C_OK,
            AgentStatus::Stale  => C_WARN,
            AgentStatus::Dead   => C_DIM,
        };
        let (priv_str, priv_color) = match a.privilege {
            Privilege::System => (" SYSTEM", C_DANGER),
            Privilege::Admin  => ("  ADMIN", C_WARN),
            Privilege::User   => ("   USER", C_FG),
        };
        // Show number in original list order +1
        let num = *real_idx + 1;
        Line::from(vec![
            Span::styled(format!(" {:>2} ", num), Style::default().fg(C_DIM)),
            Span::styled("● ", Style::default().fg(dot_color)),
            Span::styled(
                format!("{:<18}", &a.hostname),
                Style::default().fg(C_FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled(priv_str, Style::default().fg(priv_color)),
        ])
    }).map(|line| ListItem::new(line)).collect();

    // Map selected_agent (index in app.agents) to index in filtered list
    let filtered_sel = app.selected_agent.and_then(|sel| {
        filtered.iter().position(|(ri, _)| *ri == sel)
    });

    let filter_hint = if app.filter_active {
        format!(" /{}_", app.filter)
    } else if !app.filter.is_empty() {
        format!(" /{} ", app.filter)
    } else {
        String::new()
    };

    let title = if filter_hint.is_empty() {
        format!(" Agents ({}) ", filtered.len())
    } else {
        format!(" Agents ({})  {} ", filtered.len(), filter_hint)
    };

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
    let agent = app.selected_agent.and_then(|i| app.agents.get(i));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(C_BORDER))
        .title(Span::styled(
            agent.map(|a| format!(" {} ", a.hostname)).unwrap_or_else(|| " — ".into()),
            Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(a) = agent {
        let (priv_str, priv_color) = match a.privilege {
            Privilege::System => ("SYSTEM", C_DANGER),
            Privilege::Admin  => ("ADMIN",  C_WARN),
            Privilege::User   => ("USER",   C_OK),
        };
        let (status_str, status_color) = match a.status {
            AgentStatus::Active => ("ACTIVE", C_OK),
            AgentStatus::Stale  => ("STALE",  C_WARN),
            AgentStatus::Dead   => ("DEAD",   C_DANGER),
        };

        let kv = |k: &'static str, v: String| -> Line<'static> {
            Line::from(vec![
                Span::styled(format!("  {:<13}: ", k), Style::default().fg(C_LABEL)),
                Span::styled(v, Style::default().fg(C_FG)),
            ])
        };
        let kv_col = |k: &'static str, v: &'static str, color: Color| -> Line<'static> {
            Line::from(vec![
                Span::styled(format!("  {:<13}: ", k), Style::default().fg(C_LABEL)),
                Span::styled(v, Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ])
        };

        let last_seen = a.last_seen.format("%H:%M:%S").to_string();
        let first_seen = a.first_seen.format("%H:%M:%S").to_string();
        let sleep = format!("{}ms (±{}%)", a.sleep_ms, a.jitter_pct);
        let domain = a.domain.clone().unwrap_or_else(|| "—".into());

        let lines = vec![
            Line::from(""),
            kv("Hostname",   a.hostname.clone()),
            kv("Username",   a.username.clone()),
            kv("Domain",     domain),
            kv("IP",         a.ip.clone()),
            kv("OS",         a.os.clone()),
            kv("Arch",       a.arch.clone()),
            kv("PID",        a.pid.to_string()),
            kv_col("Privilege", priv_str, priv_color),
            kv_col("Status",    status_str, status_color),
            kv("Sleep",      sleep),
            kv("First Seen", first_seen),
            kv("Last Seen",  last_seen),
            Line::from(""),
            Line::from(Span::styled(
                "  ─────────────────────────────────────────────",
                Style::default().fg(C_DIM),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [Enter]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" interact   ", Style::default().fg(C_DIM)),
                Span::styled("[/]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" filter   ", Style::default().fg(C_DIM)),
                Span::styled("[↑↓]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" select", Style::default().fg(C_DIM)),
            ]),
        ];

        let para = Paragraph::new(lines);
        f.render_widget(para, inner);
    } else {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled(
                "\n  No agents connected. Build and deploy an agent to get started.",
                Style::default().fg(C_DIM),
            ),
        ]));
        f.render_widget(hint, inner);
    }
}
