use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, LineGauge, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::domain::execution::{format_duration, NodeRunStatus};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let detail = match &app.execution_detail {
        Some(d) => d.clone(),
        None => {
            let p = Paragraph::new("Loading execution…")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(block(" Execution Detail "));
            frame.render_widget(p, area);
            return;
        }
    };

    let node_runs = &app.cached_node_runs;
    let has_nodes = !node_runs.is_empty();

    // Node stats
    let total_nodes = node_runs.len();
    let success_nodes = node_runs.iter().filter(|n| n.status == NodeRunStatus::Success).count();
    let error_nodes = node_runs.iter().filter(|n| n.status == NodeRunStatus::Error).count();
    let total_dur: i64 = node_runs.iter().filter_map(|n| n.duration_ms).sum();
    let slowest = node_runs.iter().max_by_key(|n| n.duration_ms.unwrap_or(0));
    let total_items_out: usize = node_runs.iter().map(|n| n.items_out).sum();

    let waterfall_height = if has_nodes {
        (node_runs.len() as u16 + 3).min(area.height / 3)
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_nodes {
            vec![
                Constraint::Length(7),                // Header
                Constraint::Length(3),                // Node stats
                Constraint::Length(3),                // Node success gauge
                Constraint::Length(waterfall_height), // Waterfall
                Constraint::Min(1),                   // Table
            ]
        } else {
            vec![Constraint::Length(7), Constraint::Min(1)]
        })
        .split(area);

    // ── Header ──
    let status_style = match detail.status {
        crate::domain::execution::ExecutionStatus::Success => Style::default().fg(Color::Green),
        crate::domain::execution::ExecutionStatus::Error => Style::default().fg(Color::Red),
        crate::domain::execution::ExecutionStatus::Running => Style::default().fg(Color::Blue),
        crate::domain::execution::ExecutionStatus::Waiting => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    };

    let now = app.now;
    let text = vec![
        Line::from(vec![
            Span::styled("Execution: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&detail.id, Style::default().fg(Color::White).bold()),
            Span::raw("  "),
            Span::styled(format!("{} {}", detail.status.symbol(), detail.status), status_style.bold()),
        ]),
        Line::from(vec![
            Span::styled("Workflow: ", Style::default().fg(Color::DarkGray)),
            Span::raw(detail.workflow_id.as_deref().unwrap_or("—")),
            Span::raw("  "),
            Span::styled("Mode: ", Style::default().fg(Color::DarkGray)),
            Span::raw(&detail.mode),
        ]),
        Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(Color::DarkGray)),
            Span::styled(detail.duration_display(now), Style::default().fg(Color::White).bold()),
            Span::raw("  "),
            Span::styled("Started: ", Style::default().fg(Color::DarkGray)),
            Span::raw(detail.started_at.map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_else(|| "—".into())),
            Span::raw("  "),
            Span::styled("Stopped: ", Style::default().fg(Color::DarkGray)),
            Span::raw(detail.stopped_at.map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_else(|| "—".into())),
        ]),
        if let Some(ref retry_of) = detail.retry_of {
            Line::from(vec![
                Span::styled("Retry of: ", Style::default().fg(Color::DarkGray)),
                Span::styled(retry_of.as_str(), Style::default().fg(Color::Yellow)),
            ])
        } else {
            Line::from("")
        },
    ];

    let header = Paragraph::new(text)
        .block(block(" Execution Detail "))
        .wrap(Wrap { trim: false });
    frame.render_widget(header, chunks[0]);

    if !has_nodes {
        let p = Paragraph::new("No node execution data available.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(block(" Node Timeline "));
        frame.render_widget(p, chunks[1]);
        return;
    }

    // ── Node quick stats ──
    let slowest_name = slowest.map(|n| n.node_name.as_str()).unwrap_or("—");
    let slowest_dur = slowest.and_then(|n| n.duration_ms).map(format_duration).unwrap_or_else(|| "—".into());
    let node_success_pct = if total_nodes > 0 { success_nodes * 100 / total_nodes } else { 0 };

    let node_stats = Paragraph::new(Line::from(vec![
        Span::styled(" NODES ", Style::default().fg(Color::Black).bg(Color::Cyan).bold()),
        Span::styled(format!(" {} ", total_nodes), Style::default().fg(Color::White).bold()),
        Span::raw("  "),
        Span::styled(" ✓ ", Style::default().fg(Color::Black).bg(Color::Green).bold()),
        Span::styled(format!(" {} ({}%) ", success_nodes, node_success_pct), Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled(" ✗ ", Style::default().fg(Color::Black).bg(Color::Red).bold()),
        Span::styled(format!(" {} ", error_nodes), Style::default().fg(Color::Red)),
        Span::raw("  "),
        Span::styled(" TOTAL TIME ", Style::default().fg(Color::Black).bg(Color::Magenta).bold()),
        Span::styled(format!(" {} ", format_duration(total_dur)), Style::default().fg(Color::Magenta)),
        Span::raw("  "),
        Span::styled(" SLOWEST ", Style::default().fg(Color::Black).bg(Color::DarkGray).bold()),
        Span::styled(format!(" {} ({}) ", slowest_name, slowest_dur), Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(" ITEMS ", Style::default().fg(Color::Black).bg(Color::Yellow).bold()),
        Span::styled(format!(" {} out ", total_items_out), Style::default().fg(Color::Yellow)),
    ]))
    .block(block(" Node Stats "));
    frame.render_widget(node_stats, chunks[1]);

    // ── Node success gauge ──
    let node_ratio = if total_nodes > 0 { success_nodes as f64 / total_nodes as f64 } else { 0.0 };
    let gauge = LineGauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Node Success: {}% ", node_success_pct))
                .title_style(if node_success_pct == 100 {
                    Style::default().fg(Color::Green).bold()
                } else {
                    Style::default().fg(Color::Red).bold()
                })
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .filled_style(Style::default().fg(Color::Green))
        .unfilled_style(Style::default().fg(Color::Red))
        .ratio(node_ratio.min(1.0))
        .line_set(symbols::line::THICK);
    frame.render_widget(gauge, chunks[2]);

    // ── Waterfall timeline ──
    let max_dur = node_runs.iter().filter_map(|nr| nr.duration_ms).max().unwrap_or(1).max(1);
    let waterfall_width = chunks[3].width.saturating_sub(26) as usize;

    let waterfall_lines: Vec<Line> = node_runs
        .iter()
        .map(|nr| {
            let dur = nr.duration_ms.unwrap_or(0);
            let bar_len = if max_dur > 0 {
                ((dur as f64 / max_dur as f64) * waterfall_width as f64) as usize
            } else {
                0
            }
            .max(1);

            let (bar_char, bar_style) = match nr.status {
                NodeRunStatus::Success => ("█", Style::default().fg(Color::Green)),
                NodeRunStatus::Error => ("█", Style::default().fg(Color::Red)),
                NodeRunStatus::Waiting => ("░", Style::default().fg(Color::Yellow)),
                NodeRunStatus::Unknown => ("░", Style::default().fg(Color::DarkGray)),
            };

            let bar: String = bar_char.repeat(bar_len);
            let label = format!("{:>16} ", truncate_name(&nr.node_name, 16));
            let pct = if total_dur > 0 { dur * 100 / total_dur } else { 0 };
            let dur_label = format!(
                " {} ({}%)",
                nr.duration_ms.map(format_duration).unwrap_or_default(),
                pct
            );

            Line::from(vec![
                Span::styled(label, Style::default().fg(Color::White)),
                Span::styled(bar, bar_style),
                Span::styled(dur_label, Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let waterfall = Paragraph::new(waterfall_lines).block(block(" ⏱ Node Waterfall "));
    frame.render_widget(waterfall, chunks[3]);

    // ── Detail table ──
    let header_row = Row::new(vec!["", "Node", "Status", "Duration", "%", "Out", "Error"])
        .style(Style::default().fg(Color::Cyan).bold())
        .bottom_margin(1);

    let selected = app.node_scroll.selected();
    let rows: Vec<Row> = node_runs
        .iter()
        .enumerate()
        .map(|(i, nr)| {
            let status_cell = match nr.status {
                NodeRunStatus::Success => Cell::from("✓").style(Style::default().fg(Color::Green)),
                NodeRunStatus::Error => Cell::from("✗").style(Style::default().fg(Color::Red)),
                NodeRunStatus::Waiting => Cell::from("◷").style(Style::default().fg(Color::Yellow)),
                NodeRunStatus::Unknown => Cell::from("?"),
            };
            let dur = nr.duration_ms.map(format_duration).unwrap_or_else(|| "—".into());
            let pct = if total_dur > 0 {
                format!("{}%", nr.duration_ms.unwrap_or(0) * 100 / total_dur)
            } else {
                "—".into()
            };
            let error = nr.error.as_deref().map(|e| if e.len() > 35 { format!("{}…", &e[..35]) } else { e.to_string() }).unwrap_or_default();
            let style = if i == selected { Style::default().bg(Color::DarkGray).fg(Color::White) } else { Style::default() };

            Row::new(vec![
                Cell::from(format!("{:>2}.", i + 1)).style(Style::default().fg(Color::DarkGray)),
                Cell::from(nr.node_name.clone()),
                status_cell,
                Cell::from(dur),
                Cell::from(pct).style(Style::default().fg(Color::DarkGray)),
                Cell::from(format!("{}", nr.items_out)),
                Cell::from(error).style(Style::default().fg(Color::Red)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(14),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Length(4),
            Constraint::Min(15),
        ],
    )
    .header(header_row)
    .block(block(" Nodes "))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_stateful_widget(table, chunks[4], app.node_scroll.table_state_mut());
}

fn truncate_name(name: &str, max: usize) -> String {
    if name.len() <= max {
        name.to_string()
    } else {
        format!("{}…", &name[..max - 1])
    }
}

fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::DarkGray))
}
