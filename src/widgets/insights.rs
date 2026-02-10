use ratatui::prelude::*;
use ratatui::widgets::{
    Bar, BarChart, BarGroup, Block, Borders, Cell, Paragraph, Row, Table, Wrap,
};

use crate::app::{App, View};
use crate::domain::insights::Severity;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    match &app.view.clone() {
        View::InsightDetail(idx) => render_detail(app, frame, area, *idx),
        _ => render_list(app, frame, area),
    }
}

fn render_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let result = match &app.insights_result {
        Some(r) => r.clone(),
        None => {
            let msg = if app.loading {
                "Scanning…"
            } else {
                "Press r to run insights scan"
            };
            let p = Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Insights ")
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
            frame.render_widget(p, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Severity bar chart
            Constraint::Min(1),   // Findings table
        ])
        .split(area);

    // ── Severity distribution chart ──
    let bars = vec![
        Bar::default()
            .value(result.critical_count() as u64)
            .label("✗ Critical".into())
            .style(Style::default().fg(Color::Red)),
        Bar::default()
            .value(result.warning_count() as u64)
            .label("⚠ Warning".into())
            .style(Style::default().fg(Color::Yellow)),
        Bar::default()
            .value(result.info_count() as u64)
            .label("ℹ Info".into())
            .style(Style::default().fg(Color::Cyan)),
    ];

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Scanned {} workflows, {} executions ({} findings) ",
                    result.workflows_scanned,
                    result.executions_scanned,
                    result.findings.len()
                ))
                .title_style(Style::default().fg(Color::White).bold())
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(12)
        .bar_gap(2)
        .direction(Direction::Horizontal)
        .value_style(Style::default().fg(Color::White).bold());

    frame.render_widget(chart, chunks[0]);

    // ── Findings table ──
    if result.findings.is_empty() {
        let p = Paragraph::new("  No issues found. Your n8n instance looks healthy!")
            .style(Style::default().fg(Color::Green))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Findings ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(p, chunks[1]);
        return;
    }

    let header = Row::new(vec!["Sev", "Category", "Title", "Entity"])
        .style(Style::default().fg(Color::Cyan).bold())
        .bottom_margin(1);

    let selected = app.insight_scroll.selected();
    let rows: Vec<Row> = result
        .findings
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let sev_cell = match f.severity {
                Severity::Critical => Cell::from(format!("{} CRIT", f.severity.symbol()))
                    .style(Style::default().fg(Color::Red)),
                Severity::Warning => Cell::from(format!("{} WARN", f.severity.symbol()))
                    .style(Style::default().fg(Color::Yellow)),
                Severity::Info => Cell::from(format!("{} INFO", f.severity.symbol()))
                    .style(Style::default().fg(Color::Cyan)),
            };

            let style = if i == selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            Row::new(vec![
                sev_cell,
                Cell::from(f.category.to_string()),
                Cell::from(f.title.clone()),
                Cell::from(f.affected_entity.clone())
                    .style(Style::default().fg(Color::DarkGray)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(24),
            Constraint::Min(20),
            Constraint::Length(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Findings ")
            .title_style(Style::default().fg(Color::Cyan))
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_stateful_widget(table, chunks[1], app.insight_scroll.table_state_mut());
}

fn render_detail(app: &App, frame: &mut Frame, area: Rect, idx: usize) {
    let result = match &app.insights_result {
        Some(r) => r,
        None => return,
    };
    let finding = match result.findings.get(idx) {
        Some(f) => f,
        None => return,
    };

    let sev_style = match finding.severity {
        Severity::Critical => Style::default().fg(Color::Red),
        Severity::Warning => Style::default().fg(Color::Yellow),
        Severity::Info => Style::default().fg(Color::Cyan),
    };

    let text = vec![
        Line::from(Span::styled(
            format!("{} {}", finding.severity.symbol(), finding.severity),
            sev_style.bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Title:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(&finding.title, Style::default().fg(Color::White).bold()),
        ]),
        Line::from(vec![
            Span::styled("Category: ", Style::default().fg(Color::DarkGray)),
            Span::raw(finding.category.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Entity:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(&finding.affected_entity),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Detail:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(finding.detail.clone()),
        Line::from(""),
        Line::from(vec![
            Span::styled("Computed: ", Style::default().fg(Color::DarkGray)),
            Span::raw(finding.computed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        ]),
    ];

    let p = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Insight Detail ")
                .title_style(Style::default().fg(Color::Cyan))
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}
