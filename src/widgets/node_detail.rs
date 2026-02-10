use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect, node_idx: usize) {
    let node_run = match app.cached_node_runs.get(node_idx) {
        Some(nr) => nr,
        None => {
            let p = Paragraph::new("Node not found")
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Node Detail "),
                );
            frame.render_widget(p, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(1)])
        .split(area);

    let status_style = match node_run.status {
        crate::domain::execution::NodeRunStatus::Success => Style::default().fg(Color::Green),
        crate::domain::execution::NodeRunStatus::Error => Style::default().fg(Color::Red),
        _ => Style::default(),
    };

    let dur = node_run
        .duration_ms
        .map(crate::domain::execution::format_duration)
        .unwrap_or_else(|| "—".into());

    let header_text = vec![
        Line::from(vec![
            Span::styled("Node: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &node_run.node_name,
                Style::default().fg(Color::White).bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", node_run.status), status_style),
            Span::raw("  "),
            Span::styled("Duration: ", Style::default().fg(Color::DarkGray)),
            Span::raw(&dur),
            Span::raw("  "),
            Span::styled("Items: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(
                "{} in / {} out",
                node_run.items_in, node_run.items_out
            )),
        ]),
        if let Some(ref err) = node_run.error {
            Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::styled(err.as_str(), Style::default().fg(Color::Red)),
            ])
        } else {
            Line::from("")
        },
    ];

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Node Detail ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(header, chunks[0]);

    let json_str = node_run
        .data
        .as_ref()
        .map(|d| serde_json::to_string_pretty(d).unwrap_or_else(|_| "{}".into()))
        .unwrap_or_else(|| "No data available".into());

    let json_widget = Paragraph::new(json_str)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Data (JSON) ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    frame.render_widget(json_widget, chunks[1]);
}
