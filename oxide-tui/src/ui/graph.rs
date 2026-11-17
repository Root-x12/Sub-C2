use crate::app::{App, NodeKind};
use crate::ui::{C_ACCENT, C_BORDER, C_BG, C_DANGER, C_DIM, C_OK, C_WARN};
use oxide_common::types::AgentStatus;
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

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_BORDER))
                .style(Style::default().bg(C_BG))
                .title(Span::styled(
                    " Network ",
                    Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                )),
        )
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0])
        .paint(move |ctx| {
            // Draw edges
            if let Some(server) = nodes.first() {
                for node in nodes.iter().skip(1) {
                    let edge_color = match &node.kind {
                        NodeKind::Agent { status, .. } => match status {
                            AgentStatus::Active => Color::DarkGray,
                            AgentStatus::Stale  => Color::Yellow,
                            AgentStatus::Dead   => Color::Red,
                        },
                        _ => Color::DarkGray,
                    };
                    ctx.draw(&Line {
                        x1: server.x, y1: server.y,
                        x2: node.x,   y2: node.y,
                        color: edge_color,
                    });
                }
            }

            // Draw nodes
            for node in nodes.iter() {
                match &node.kind {
                    NodeKind::Server => {
                        // Double ring: outer dim ring + inner accent
                        ctx.draw(&Circle { x: node.x, y: node.y, radius: 3.5, color: C_DIM });
                        ctx.draw(&Circle { x: node.x, y: node.y, radius: 2.2, color: C_ACCENT });
                        ctx.print(
                            node.x - 3.0,
                            node.y - 5.5,
                            Span::styled("OXIDE", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                        );
                        if agent_count == 0 {
                            ctx.print(
                                node.x - 9.0,
                                node.y - 7.5,
                                Span::styled("no agents connected", Style::default().fg(C_DIM)),
                            );
                        }
                    }
                    NodeKind::Agent { status, priv_lvl } => {
                        let color = match status {
                            AgentStatus::Active => match priv_lvl {
                                oxide_common::types::Privilege::System => C_DANGER,
                                oxide_common::types::Privilege::Admin  => C_WARN,
                                oxide_common::types::Privilege::User   => C_OK,
                            },
                            AgentStatus::Stale => Color::Yellow,
                            AgentStatus::Dead  => Color::DarkGray,
                        };
                        ctx.draw(&Circle { x: node.x, y: node.y, radius: 1.5, color });
                        let label = node.label.lines().next().unwrap_or("?").to_string();
                        ctx.print(
                            node.x - label.len() as f64 / 4.0,
                            node.y - 3.2,
                            Span::styled(label, Style::default().fg(color)),
                        );
                    }
                }
            }
        });

    f.render_widget(canvas, area);
}
