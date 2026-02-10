use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, LineGauge, Paragraph, Row, Sparkline, Table};

use crate::app::{App, SortKind};
use crate::domain::execution::ExecutionStatus;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let executions = app.filtered_executions();
    let now = app.now;

    if executions.is_empty() && !app.loading {
        let msg = if app.exec_status_filter.is_none() && app.exec_filter_text.is_empty() {
            "No executions found. Press r to refresh."
        } else {
            "No executions match current filters. Press 0 to clear."
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(block(" Executions "));
        frame.render_widget(p, area);
        return;
    }

    let c = &app.exec_counts;
    let total = c.total.max(1) as f64;
    let success_pct = (c.success as f64 / total * 100.0) as u16;
    let error_pct = (c.error as f64 / total * 100.0) as u16;

    // Compute avg/max duration
    let durations: Vec<i64> = executions
        .iter()
        .filter_map(|e| {
            e.stopped_at
                .zip(e.started_at)
                .map(|(stop, start)| (stop - start).num_milliseconds())
        })
        .collect();
    let avg_dur_ms = if durations.is_empty() { 0 } else { durations.iter().sum::<i64>() / durations.len() as i64 };
    let max_dur_ms = durations.iter().max().copied().unwrap_or(0);
    let retries = executions.iter().filter(|e| e.retry_of.is_some()).count();

    let filter_info = app
        .exec_status_filter
        .as_ref()
        .map(|s| format!(" [filter: {}]", s))
        .unwrap_or_default();

    // Build inline mini-bars for stats
    let success_bar = "█".repeat((success_pct as usize / 5).max(if c.success > 0 { 1 } else { 0 }));
    let error_bar = "█".repeat((error_pct as usize / 5).max(if c.error > 0 { 1 } else { 0 }));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Quick stats
            Constraint::Length(3), // Success rate gauge
            Constraint::Length(5), // Sparkline (wider)
            Constraint::Min(1),   // Table
        ])
        .split(area);

    // ── Quick stats ──
    let stats = Paragraph::new(Line::from(vec![
        Span::styled(" TOTAL ", Style::default().fg(Color::Black).bg(Color::Cyan).bold()),
        Span::styled(format!(" {} ", c.total), Style::default().fg(Color::White).bold()),
        Span::raw("  "),
        Span::styled(" ✓ ", Style::default().fg(Color::Black).bg(Color::Green).bold()),
        Span::styled(format!(" {} ({}%) ", c.success, success_pct), Style::default().fg(Color::Green)),
        Span::styled(&success_bar, Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled(" ✗ ", Style::default().fg(Color::Black).bg(Color::Red).bold()),
        Span::styled(format!(" {} ({}%) ", c.error, error_pct), Style::default().fg(Color::Red)),
        Span::styled(&error_bar, Style::default().fg(Color::Red)),
        Span::raw("  "),
        Span::styled(" ⟳ ", Style::default().fg(Color::Black).bg(Color::Blue).bold()),
        Span::styled(format!(" {} ", c.running), Style::default().fg(Color::Blue)),
        Span::raw("  "),
        Span::styled(" AVG ", Style::default().fg(Color::Black).bg(Color::Magenta).bold()),
        Span::styled(format!(" {} ", crate::domain::execution::format_duration(avg_dur_ms)), Style::default().fg(Color::Magenta)),
        Span::raw(" "),
        Span::styled(" MAX ", Style::default().fg(Color::Black).bg(Color::DarkGray).bold()),
        Span::styled(format!(" {} ", crate::domain::execution::format_duration(max_dur_ms)), Style::default().fg(Color::White)),
        Span::raw(" "),
        Span::styled(" ↻ ", Style::default().fg(Color::Black).bg(Color::Yellow).bold()),
        Span::styled(format!(" {} ", retries), Style::default().fg(Color::Yellow)),
        Span::styled(filter_info, Style::default().fg(Color::DarkGray)),
    ]))
    .block(block(" Quick Stats "));
    frame.render_widget(stats, chunks[0]);

    // ── Success rate gauge (full width) ──
    let success_ratio = (c.success as f64 / total).min(1.0);
    let gauge = LineGauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Success Rate: {}% ({}/{})  ·  Error Rate: {}% ({}/{}) ",
                    success_pct, c.success, c.total, error_pct, c.error, c.total
                ))
                .title_style(if success_pct >= 80 {
                    Style::default().fg(Color::Green).bold()
                } else if success_pct >= 50 {
                    Style::default().fg(Color::Yellow).bold()
                } else {
                    Style::default().fg(Color::Red).bold()
                })
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .filled_style(Style::default().fg(Color::Green))
        .unfilled_style(Style::default().fg(Color::Red))
        .ratio(success_ratio)
        .line_set(symbols::line::THICK);
    frame.render_widget(gauge, chunks[1]);

    // ── Sparkline (fill the entire width: one data point per column) ──
    let sparkline_width = chunks[2].width.saturating_sub(2) as usize; // minus borders
    let sparkline_data = build_execution_sparkline(&executions, now, sparkline_width);
    let peak = sparkline_data.iter().max().copied().unwrap_or(0);
    let mins = sparkline_width; // each column = 1 minute
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Execution Frequency (last {}min · peak: {}/min) ", mins, peak))
                .title_style(Style::default().fg(Color::Cyan))
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .data(&sparkline_data)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(sparkline, chunks[2]);

    // ── Table ──
    let sort = &app.exec_sort;
    let status_hdr = format!("Status{}", app.sort_indicator(sort, &SortKind::StatusAsc, &SortKind::StatusDesc));
    let wf_hdr = format!("Workflow{}", app.sort_indicator(sort, &SortKind::NameAsc, &SortKind::NameDesc));
    let dur_hdr = format!("Duration{}", app.sort_indicator(sort, &SortKind::DurationAsc, &SortKind::DurationDesc));

    let header = Row::new(vec!["ID", &status_hdr, &wf_hdr, "Mode", "Started", &dur_hdr])
        .style(Style::default().fg(Color::Cyan).bold())
        .bottom_margin(1);

    let selected = app.exec_scroll.selected();
    let rows: Vec<Row> = executions
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let status_cell = match e.status {
                ExecutionStatus::Success => Cell::from(format!("{} Success", e.status.symbol())).style(Style::default().fg(Color::Green)),
                ExecutionStatus::Error => Cell::from(format!("{} Error", e.status.symbol())).style(Style::default().fg(Color::Red)),
                ExecutionStatus::Running => Cell::from(format!("{} Running", e.status.symbol())).style(Style::default().fg(Color::Blue)),
                ExecutionStatus::Waiting => Cell::from(format!("{} Waiting", e.status.symbol())).style(Style::default().fg(Color::Yellow)),
                ExecutionStatus::Canceled => Cell::from(format!("{} Canceled", e.status.symbol())).style(Style::default().fg(Color::DarkGray)),
                ExecutionStatus::Unknown => Cell::from("? Unknown"),
            };

            let wf_name = e.workflow_name.as_deref().or(e.workflow_id.as_deref()).unwrap_or("—");
            let retry_indicator = if e.retry_of.is_some() { " ↻" } else { "" };
            let style = if i == selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(e.id.clone()).style(Style::default().fg(Color::DarkGray)),
                status_cell,
                Cell::from(format!("{}{}", wf_name, retry_indicator)),
                Cell::from(e.mode.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(e.started_ago(now)),
                Cell::from(e.duration_display(now)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(block(" Executions "))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_stateful_widget(table, chunks[3], app.exec_scroll.table_state_mut());
}

fn build_execution_sparkline(
    executions: &[crate::domain::execution::ExecutionSummary],
    now: chrono::DateTime<chrono::Utc>,
    buckets: usize,
) -> Vec<u64> {
    let mut data = vec![0u64; buckets];
    for e in executions {
        if let Some(started) = e.started_at {
            let mins_ago = (now - started).num_minutes();
            if mins_ago >= 0 && (mins_ago as usize) < buckets {
                let idx = buckets - 1 - (mins_ago as usize);
                data[idx] += 1;
            }
        }
    }
    data
}

fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::DarkGray))
}
