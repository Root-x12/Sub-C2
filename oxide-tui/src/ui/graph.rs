use crate::app::{App, NodeKind};
use crate::ui::{C_ACCENT, C_BORDER, C_BG, C_DANGER, C_DIM, C_OK, C_SELECT, C_WARN};
use oxide_common::types::{AgentStatus, Privilege};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{
        canvas::{Canvas, Circle, Line},
        Block, Borders,
    },
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let nodes = &app.graph_nodes;
    let agent_count = nodes.len().saturating_sub(1);
    let tick = app.tick_count;

    // Animated pulse: outer ring alternates between two radii every 4 ticks
    let pulse = (tick / 4) % 2 == 0;
    let outer_r  = if pulse { 4.0 } else { 3.5 };
    let inner_r  = if pulse { 2.5 } else { 2.2 };
    let ring_col = if pulse { C_ACCENT } else { C_DIM };

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_BORDER))
                .style(Style::default().bg(C_BG))
                .title(Span::styled(
                    " ◈ Network ",
                    Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                )),
        )
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0])
        .paint(move |ctx| {
            // Draw edges first (behind nodes)
            if let Some(server) = nodes.first() {
                for node in nodes.iter().skip(1) {
                    let edge_color = match &node.kind {
                        NodeKind::Agent { status, .. } => match status {
                            AgentStatus::Active => Color::Rgb(50, 80, 60),
                            AgentStatus::Stale  => Color::Rgb(80, 70, 20),
                            AgentStatus::Dead   => Color::Rgb(60, 30, 30),
                        },
                        _ => Color::DarkGray,
                    };
                    ctx.draw(&Line {
                        x1: server.x, y1: server.y,
                        x2: node.x,   y2: node.y,
                        color: edge_color,
                    });
                    // Midpoint label showing status
                    let mx = (server.x + node.x) / 2.0;
                    let my = (server.y + node.y) / 2.0;
                    if let NodeKind::Agent { status, .. } = &node.kind {
                        let (sym, col) = match status {
                            AgentStatus::Active => ("▸", C_OK),
                            AgentStatus::Stale  => ("?", C_WARN),
                            AgentStatus::Dead   => ("✗", Color::DarkGray),
                        };
                        ctx.print(mx, my, Span::styled(sym, Style::default().fg(col)));
                    }
                }
            }

            // Draw nodes
            for node in nodes.iter() {
                match &node.kind {
                    NodeKind::Server => {
                        // Animated double ring
                        ctx.draw(&Circle { x: node.x, y: node.y, radius: outer_r, color: ring_col });
                        ctx.draw(&Circle { x: node.x, y: node.y, radius: inner_r, color: C_ACCENT });
                        ctx.print(
                            node.x - 3.0,
                            node.y + 6.0,
                            Span::styled("◈ OXIDE", Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)),
                        );
                        ctx.print(
                            node.x - 2.5,
                            node.y + 4.0,
                            Span::styled("C2 SERVER", Style::default().fg(C_DIM)),
                        );
                        if agent_count == 0 {
                            ctx.print(
                                node.x - 10.0,
                                node.y - 7.0,
                                Span::styled("awaiting agents…", Style::default().fg(C_DIM)),
                            );
                        }
                    }
                    NodeKind::Agent { status, priv_lvl } => {
                        let (node_color, ring_color) = match status {
                            AgentStatus::Active => match priv_lvl {
                                Privilege::System => (C_DANGER, Color::Rgb(120, 30, 30)),
                                Privilege::Admin  => (C_WARN,   Color::Rgb(100, 80, 20)),
                                Privilege::User   => (C_OK,     Color::Rgb(30, 80, 30)),
                            },
                            AgentStatus::Stale => (C_WARN, Color::DarkGray),
                            AgentStatus::Dead  => (Color::DarkGray, Color::Rgb(40, 40, 40)),
                        };
                        // Outer hint ring
                        ctx.draw(&Circle { x: node.x, y: node.y, radius: 2.2, color: ring_color });
                        ctx.draw(&Circle { x: node.x, y: node.y, radius: 1.4, color: node_color });

                        // Hostname label
                        let label = node.label.lines().next().unwrap_or("?").to_string();
                        let lx = node.x - label.len() as f64 / 4.0;
                        ctx.print(
                            lx,
                            node.y - 4.0,
                            Span::styled(label, Style::default().fg(node_color).add_modifier(Modifier::BOLD)),
                        );
                        // Privilege badge below label
                        let (badge, badge_col) = match priv_lvl {
                            Privilege::System => ("SYS", C_DANGER),
                            Privilege::Admin  => ("ADM", C_WARN),
                            Privilege::User   => ("USR", C_OK),
                        };
                        ctx.print(
                            node.x - 1.5,
                            node.y - 6.0,
                            Span::styled(badge, Style::default().fg(badge_col)),
                        );
                    }
                }
            }
        });

    f.render_widget(canvas, area);
}
