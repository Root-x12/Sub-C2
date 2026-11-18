use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_BG, C_DIM, C_DANGER, C_FG, C_OK, C_SELECT, C_WARN, graph};
use oxide_common::types::{AgentStatus, ListenerStatus, Privilege};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Sparkline},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    // Outer split: top 55% / bottom 45%
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    // Top: stats (22%) | graph (78%)
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(22), Constraint::Percentage(78)])
        .split(vert[0]);

    // Bottom: log (66%) | right column (34%)
    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(vert[1]);

    // Right column: sparkline (5 rows) + agents list
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(bot[1]);

    draw_stats(f, app, top[0]);
    graph::draw(f, app, top[1]);
    draw_log(f, app, bot[0]);
    draw_sparkline(f, app, right[0]);
    draw_agents(f, app, right[1]);
}

fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let total   = app.agents.len();
    let active  = app.agents.iter().filter(|a| a.status == AgentStatus::Active).count();
    let stale   = app.agents.iter().filter(|a| a.status == AgentStatus::Stale).count();
    let dead    = app.agents.iter().filter(|a| a.status == AgentStatus::Dead).count();
    let systems = app.agents.iter().filter(|a| a.privilege == Privilege::System).count();
    let admins  = app.agents.iter().filter(|a| a.privilege == Privilege::Admin).count();
    let users   = app.agents.iter().filter(|a| a.privilege == Privilege::User).count();
    let running = app.listeners.iter().filter(|l| l.status == ListenerStatus::Running).count();
    let loot_n  = app.loot.len();

    // Threat bar: filled blocks based on SYS + ADM counts (max width 8)
    let threat_score = (systems * 3 + admins).min(8);
    let threat_color = if systems > 0 { C_DANGER } else if admins > 0 { C_WARN } else { C_DIM };
    let threat_bar: String = "█".repeat(threat_score) + &"░".repeat(8usize.saturating_sub(threat_score));

    let sep = Line::from(Span::styled("  ─────────────────", Style::default().fg(C_DIM)));

    let header = |label: &'static str| -> Line<'static> {
        Line::from(Span::styled(
            format!("  {label}"),
            Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ))
    };

    let row = |label: &'static str, val: usize, color: Color| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<10}", label), Style::default().fg(C_DIM)),
            Span::styled(format!("{:>3}", val), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ])
    };

    let lines: Vec<Line> = vec![
        Line::from(""),
        header("AGENTS"),
        row("Active",   active,  if active > 0 { C_OK } else { C_DIM }),
        row("Stale",    stale,   if stale  > 0 { C_WARN } else { C_DIM }),
        row("Dead",     dead,    if dead   > 0 { C_DANGER } else { C_DIM }),
        row("Total",    total,   C_FG),
        sep.clone(),
        header("PRIVILEGE"),
        row("SYSTEM",   systems, if systems > 0 { C_DANGER } else { C_DIM }),
        row("Admin",    admins,  if admins  > 0 { C_WARN }   else { C_DIM }),
        row("User",     users,   if users   > 0 { C_OK }     else { C_DIM }),
        sep.clone(),
        Line::from(vec![
            Span::styled("  Threat  ", Style::default().fg(C_DIM)),
            Span::styled(threat_bar, Style::default().fg(threat_color)),
        ]),
        sep.clone(),
        header("INFRA"),
        row("Listeners", running, C_ACCENT),
        row("Loot",      loot_n,  C_ACCENT),
        Line::from(""),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG))
            .title(Span::styled(
                " ◈ Stats ",
                Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
            )));
    f.render_widget(para, area);
}

fn draw_sparkline(f: &mut Frame, app: &App, area: Rect) {
    let data: Vec<u64> = app.beacon_history.iter().copied().collect();
    let peak = data.iter().copied().max().unwrap_or(0);
    let current = data.last().copied().unwrap_or(0);

    let title = format!(" Beacons/s  now:{current}  peak:{peak} ");

    let sparkline = Sparkline::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG))
            .title(Span::styled(title, Style::default().fg(C_ACCENT))))
        .data(&data)
        .style(Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD));
    f.render_widget(sparkline, area);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    // Show log newest-first (reverse), limited to visible height
    let log_items: Vec<_> = app.log.iter().collect();
    let items: Vec<ListItem> = log_items.iter().rev().map(|msg| {
        let (ts, body) = if msg.starts_with('[') {
            msg.split_once("] ")
                .map(|(t, r)| (format!("{t}]"), r.to_string()))
                .unwrap_or_else(|| (String::new(), msg.to_string()))
        } else {
            (String::new(), msg.to_string())
        };

        let (prefix_sym, prefix_color, body_color) =
            if body.contains("NEW AGENT") || body.starts_with("✓") {
                ("★ ", C_OK, C_OK)
            } else if body.contains("dead") || body.contains("FAILED") || body.contains("ERROR") || body.starts_with("✗") {
                ("✗ ", C_DANGER, C_DANGER)
            } else if body.starts_with("⚠") {
                ("⚠ ", C_WARN, C_WARN)
            } else if body.starts_with("←") {
                ("← ", C_OK, C_FG)
            } else if body.starts_with("→") {
                ("→ ", C_ACCENT, C_ACCENT)
            } else if body.starts_with("build") {
                ("▸ ", C_WARN, C_WARN)
            } else {
                ("  ", C_DIM, C_DIM)
            };

        // Strip the leading arrow from body since we show it as prefix_sym
        let display_body = if body.starts_with("← ") || body.starts_with("→ ") || body.starts_with("⚠ ") || body.starts_with("✓ ") {
            body[4..].to_string()  // skip the 2-byte symbol + space
        } else {
            body.clone()
        };

        ListItem::new(Line::from(vec![
            Span::styled(ts,           Style::default().fg(C_DIM)),
            Span::styled(" ", Style::default()),
            Span::styled(prefix_sym,   Style::default().fg(prefix_color).add_modifier(Modifier::BOLD)),
            Span::styled(display_body, Style::default().fg(body_color)),
        ]))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG))
            .title(Span::styled(
                " ◈ Activity Log ",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )));
    f.render_widget(list, area);
}

fn draw_agents(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.agents.iter().map(|a| {
        let (dot, dot_color) = match a.status {
            AgentStatus::Active => ("● ", C_OK),
            AgentStatus::Stale  => ("◌ ", C_WARN),
            AgentStatus::Dead   => ("○ ", C_DIM),
        };
        let (badge, badge_color) = match a.privilege {
            Privilege::System => ("[SYS]", C_DANGER),
            Privilege::Admin  => ("[ADM]", C_WARN),
            Privilege::User   => ("[USR]", C_FG),
        };
        let last_octet = a.ip.rsplit('.').next().unwrap_or("?");
        ListItem::new(Line::from(vec![
            Span::styled(dot,  Style::default().fg(dot_color).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:<12}", &a.hostname[..a.hostname.len().min(12)]),
                Style::default().fg(C_FG),
            ),
            Span::styled(badge, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" .{last_octet}"), Style::default().fg(C_DIM)),
        ]))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG))
            .title(Span::styled(
                " ◈ Agents ",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )));
    f.render_widget(list, area);
}
