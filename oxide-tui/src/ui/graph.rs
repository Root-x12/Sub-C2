//! Canvas-based network graph — server node at center, agents around it.

use crate::app::{App, NodeKind};
use crate::ui::{C_ACCENT, C_BORDER, C_DANGER, C_DIM, C_OK, C_WARN};
use oxide_common::types::AgentStatus;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols,
    widgets::{
        canvas::{Canvas, Circle, Line, Points},
        Block, Borders,
    },
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let nodes = &app.graph_nodes;

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(C_BORDER))
                .title(" Network Graph "),
        )
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0])
        .paint(|ctx| {
            // Draw edges (lines from server to each agent)
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
                let (color, radius) = match &node.kind {
                    NodeKind::Server => (C_ACCENT, 2.5),
                    NodeKind::Agent { status, priv_lvl } => {
                        let c = match status {
                            AgentStatus::Active => match priv_lvl {
                                oxide_common::types::Privilege::System => C_DANGER,
                                oxide_common::types::Privilege::Admin  => C_WARN,
                                oxide_common::types::Privilege::User   => C_OK,
                            },
                            AgentStatus::Stale => Color::Yellow,
                            AgentStatus::Dead  => Color::DarkGray,
                        };
                        (c, 1.5)
                    }
                };

                ctx.draw(&Circle {
                    x: node.x,
                    y: node.y,
                    radius,
                    color,
                });

                // Label below node
                let label = node.label.lines().next().unwrap_or("?").to_string();
                ctx.print(node.x - label.len() as f64 / 4.0,
                          node.y - radius - 1.5,
                          ratatui::text::Span::styled(
                              label,
                              Style::default().fg(color),
                          ));
            }
        });

    f.render_widget(canvas, area);
}
