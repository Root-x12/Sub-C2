use crate::app::App;
use crate::ui::{C_ACCENT, C_BORDER, C_DIM, C_FG};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.log.iter().take(100).map(|msg| {
        ListItem::new(Line::from(Span::styled(msg.as_str(), Style::default().fg(C_FG))))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .title(" Sessions / Output Log "));
    f.render_widget(list, area);
}
