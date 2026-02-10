use ratatui::prelude::*;
use ratatui::widgets::{
    Bar, BarChart, BarGroup, Block, Borders, Cell, LineGauge, Paragraph, Row, Table,
};

use crate::app::{App, SortKind};
use crate::domain::execution::format_relative;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let workflows = app.filtered_workflows();
    let now = app.now;

    if workflows.is_empty() && !app.loading {
        let msg = if app.wf_filter_text.is_empty() && app.wf_active_filter.is_none() {
            "No workflows found. Press r to refresh."
        } else {
            "No workflows match current filters. Press 0 to clear."
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(block(" Workflows "));
        frame.render_widget(p, area);
        return;
    }

    let total = workflows.len();
    let active_count = workflows.iter().filter(|w| w.active).count();
    let inactive_count = total - active_count;
    let active_pct = if total > 0 { active_count * 100 / total } else { 0 };
    let inactive_pct = 100 - active_pct;
    let with_tags = workflows.iter().filter(|w| !w.tags.is_empty()).count();
    let filter_info = if app.wf_active_filter.is_some() || !app.wf_filter_text.is_empty() {
        format!(" (filtered from {})", app.workflows.len())
    } else {
        String::new()
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Quick stats
            Constraint::Length(3), // Active rate gauge
            Constraint::Min(1),   // Table
        ])
        .split(area);

    // ── Quick stats with inline visual bars ──
    let active_bar = "█".repeat((active_pct / 5).max(1));
    let inactive_bar = "█".repeat((inactive_pct / 5).max(1));

    let stats = Paragraph::new(Line::from(vec![
        Span::styled(" TOTAL ", Style::default().fg(Color::Black).bg(Color::Cyan).bold()),
        Span::styled(format!(" {} ", total), Style::default().fg(Color::White).bold()),
        Span::raw("  "),
        Span::styled(" ACTIVE ", Style::default().fg(Color::Black).bg(Color::Green).bold()),
        Span::styled(format!(" {} ", active_count), Style::default().fg(Color::Green).bold()),
        Span::styled(format!("({}%) ", active_pct), Style::default().fg(Color::DarkGray)),
        Span::styled(&active_bar, Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled(" INACTIVE ", Style::default().fg(Color::Black).bg(Color::DarkGray).bold()),
        Span::styled(format!(" {} ", inactive_count), Style::default().fg(Color::White)),
        Span::styled(format!("({}%) ", inactive_pct), Style::default().fg(Color::DarkGray)),
        Span::styled(&inactive_bar, Style::default().fg(Color::Red)),
        Span::raw("  "),
        Span::styled(" TAGGED ", Style::default().fg(Color::Black).bg(Color::Yellow).bold()),
        Span::styled(format!(" {} ", with_tags), Style::default().fg(Color::Yellow)),
        Span::styled(filter_info, Style::default().fg(Color::DarkGray)),
    ]))
    .block(block(" Quick Stats "));
    frame.render_widget(stats, chunks[0]);

    // ── Active ratio gauge ──
    let ratio = if total > 0 { active_count as f64 / total as f64 } else { 0.0 };
    let gauge = LineGauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Active Rate: {}% ({}/{}) ", active_pct, active_count, total))
                .title_style(Style::default().fg(Color::Green).bold())
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .filled_style(Style::default().fg(Color::Green))
        .unfilled_style(Style::default().fg(Color::DarkGray))
        .ratio(ratio)
        .line_set(symbols::line::THICK);
    frame.render_widget(gauge, chunks[1]);

    // ── Table ──
    let sort = &app.wf_sort;
    let name_hdr = format!("Name{}", app.sort_indicator(sort, &SortKind::NameAsc, &SortKind::NameDesc));
    let active_hdr = format!("Active{}", app.sort_indicator(sort, &SortKind::StatusAsc, &SortKind::StatusDesc));
    let updated_hdr = format!("Updated{}", app.sort_indicator(sort, &SortKind::UpdatedAsc, &SortKind::UpdatedDesc));

    let header = Row::new(vec!["ID", &name_hdr, &active_hdr, "Tags", &updated_hdr])
        .style(Style::default().fg(Color::Cyan).bold())
        .bottom_margin(1);

    let selected = app.wf_scroll.selected();
    let rows: Vec<Row> = workflows
        .iter()
        .enumerate()
        .map(|(i, wf)| {
            let active_cell = if wf.active {
                Cell::from("● active").style(Style::default().fg(Color::Green))
            } else {
                Cell::from("○ inactive").style(Style::default().fg(Color::DarkGray))
            };
            let updated = wf.updated_at.map(|t| format_relative(t, now)).unwrap_or_else(|| "—".into());
            let style = if i == selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(wf.id.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(wf.name.clone()),
                active_cell,
                Cell::from(wf.tag_names()).style(Style::default().fg(Color::Yellow)),
                Cell::from(updated),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Min(20),
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(block(" Workflows "))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_stateful_widget(table, chunks[2], app.wf_scroll.table_state_mut());
}

fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::DarkGray))
}
