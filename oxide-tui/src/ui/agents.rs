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
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_list(f, app, panes[0]);
    draw_detail(f, app, panes[1]);
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    // Split: search bar (3 lines) + list (rest)
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

    // ── Filtered agent list ───────────────────────────────────────────────────
    let filter = app.filter.to_lowercase();
    let filtered: Vec<(usize, &oxide_common::types::AgentInfo)> = app.agents
        .iter()
        .enumerate()
        .filter(|(_, a)| filter.is_empty() || a.hostname.to_lowercase().contains(&filter))
        .collect();

    let items: Vec<ListItem> = filtered.iter().map(|(real_idx, a)| {
        let (priv_str, _) = match a.privilege {
            Privilege::System => ("  SYSTEM", C_DANGER),
            Privilege::Admin  => ("   ADMIN", C_WARN),
            Privilege::User   => ("    USER", C_FG),
        };
        let dot = match a.status {
            AgentStatus::Active => "●",
            AgentStatus::Stale  => "◑",
            AgentStatus::Dead   => "○",
        };
        let num = real_idx + 1;
        Line::from(vec![
            Span::styled(format!("  {:>2}  {dot}  ", num), Style::default().fg(C_DIM)),
            Span::styled(format!("{:<20}", &a.hostname), Style::default().fg(C_FG)),
            Span::styled(priv_str, Style::default().fg(C_DIM)),
        ])
    }).map(ListItem::new).collect();

    let filtered_sel = app.selected_agent.and_then(|sel| {
        filtered.iter().position(|(ri, _)| *ri == sel)
    });

    // parui style: bold red foreground on selected, no background
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
    let agent = app.selected_agent.and_then(|i| app.agents.get(i));

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

        // Build key-value lines (parui style: label then value)
        let kv = |k: &'static str, v: String| -> Line<'static> {
            Line::from(vec![
                Span::styled(format!("  {:<14}", k), Style::default().fg(C_LABEL).add_modifier(Modifier::BOLD)),
                Span::styled(" :  ", Style::default().fg(C_DIM)),
                Span::styled(v, Style::default().fg(C_FG)),
            ])
        };
        let kv_col = |k: &'static str, v: &'static str, color: Color| -> Line<'static> {
            Line::from(vec![
                Span::styled(format!("  {:<14}", k), Style::default().fg(C_LABEL).add_modifier(Modifier::BOLD)),
                Span::styled(" :  ", Style::default().fg(C_DIM)),
                Span::styled(v, Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ])
        };

        let last_seen  = a.last_seen.format("%H:%M:%S").to_string();
        let first_seen = a.first_seen.format("%H:%M:%S").to_string();
        let sleep      = format!("{}ms  (±{}% jitter)", a.sleep_ms, a.jitter_pct);
        let domain     = a.domain.clone().unwrap_or_else(|| "—".into());
        let missed     = a.missed_beats.to_string();

        let sep = Span::styled(
            "  ──────────────────────────────────────────────────────",
            Style::default().fg(C_DIM),
        );

        let lines = vec![
            Line::from(""),
            // Title row: hostname large
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&a.hostname, Style::default().fg(C_SELECT)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(format!("  @  {}", a.ip), Style::default().fg(C_DIM)),
            ]),
            Line::from(""),
            Line::from(sep.clone()),
            Line::from(""),
            kv("Hostname",     a.hostname.clone()),
            kv("Username",     a.username.clone()),
            kv("Domain",       domain),
            kv("IP Address",   a.ip.clone()),
            kv("OS",           a.os.clone()),
            kv("Arch",         a.arch.clone()),
            kv("PID",          a.pid.to_string()),
            kv_col("Privilege",  priv_str,   priv_color),
            kv_col("Status",     status_str, status_color),
            kv("Sleep",        sleep),
            kv("Missed Beats", missed),
            kv("First Seen",   first_seen),
            kv("Last Seen",    last_seen),
            Line::from(""),
            Line::from(sep.clone()),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[Enter]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" interact   ", Style::default().fg(C_DIM)),
                Span::styled("[/]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" search   ", Style::default().fg(C_DIM)),
                Span::styled("[↑↓]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" select", Style::default().fg(C_DIM)),
            ]),
        ];

        // parui: no border on detail card, just floating text
        let para = Paragraph::new(lines);
        f.render_widget(para, area);
    } else {
        let lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  No agents connected.",
                Style::default().fg(C_DIM),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Deploy an agent and it will appear here. ", Style::default().fg(C_DIM)),
                Span::styled("[6]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" Build tab → ", Style::default().fg(C_DIM)),
                Span::styled("[G]", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" Generate Payload", Style::default().fg(C_DIM)),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), area);
    }
}
