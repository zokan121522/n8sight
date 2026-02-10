use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Tabs as RataTabs};

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let titles = vec![" ⚙ Workflows ", " ▶ Executions ", " ⚡ Insights "];

    let tabs = RataTabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" n8sight ")
                .title_style(Style::default().fg(Color::Cyan).bold())
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(app.active_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .bold()
                .add_modifier(Modifier::UNDERLINED),
        )
        .divider("│");

    frame.render_widget(tabs, area);
}
