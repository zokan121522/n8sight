use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::collections::HashMap;

use crate::app::App;
use crate::domain::workflow::WorkflowDetail;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let detail = match &app.workflow_detail {
        Some(d) => d,
        None => return,
    };

    let node = match detail.nodes.get(app.graph_selected_node) {
        Some(n) => n,
        None => {
            let p = Paragraph::new("No node selected")
                .style(Style::default().fg(Color::DarkGray))
                .block(block(" Node Inspect "));
            frame.render_widget(p, area);
            return;
        }
    };

    let connections = parse_connections(&detail.connections);
    let total_nodes = detail.nodes.len();
    let node_idx = app.graph_selected_node;

    // Figure out what this node connects to and what connects to it
    let outgoing: Vec<&str> = connections
        .get(node.name.as_str())
        .map(|targets| targets.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let incoming: Vec<&str> = connections
        .iter()
        .filter(|(_, targets)| targets.iter().any(|t| t == &node.name))
        .map(|(src, _)| src.as_str())
        .collect();

    let short_type = WorkflowDetail::short_node_type(&node.node_type);

    let is_trigger = node.node_type.contains("trigger")
        || node.node_type.contains("webhook")
        || node.node_type.contains("cron")
        || node.node_type.contains("schedule");

    // Layout: header, connections, parameters, credentials
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Node header
            Constraint::Length(6),  // Connections
            Constraint::Min(1),    // Parameters JSON
        ])
        .split(area);

    // ── Node header ──
    let type_style = if is_trigger {
        Style::default().fg(Color::Yellow).bold()
    } else if node.disabled {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let status_badge = if node.disabled {
        Span::styled(" DISABLED ", Style::default().fg(Color::Black).bg(Color::Red).bold())
    } else if is_trigger {
        Span::styled(" TRIGGER ", Style::default().fg(Color::Black).bg(Color::Yellow).bold())
    } else {
        Span::styled(" NODE ", Style::default().fg(Color::Black).bg(Color::Cyan).bold())
    };

    let position_str = if node.position.len() >= 2 {
        format!("({}, {})", node.position[0] as i32, node.position[1] as i32)
    } else {
        "—".to_string()
    };

    let creds = node
        .credentials
        .as_ref()
        .and_then(|c| c.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    format!("{}: {}", k, name)
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "None".to_string());

    let header_text = vec![
        Line::from(vec![
            status_badge,
            Span::raw("  "),
            Span::styled(&node.name, Style::default().fg(Color::White).bold()),
            Span::styled(
                format!("  ({}/{})", node_idx + 1, total_nodes),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("Type:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(&node.node_type, type_style),
            Span::styled(format!("  ({})", short_type), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Position:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(&position_str),
        ]),
        Line::from(vec![
            Span::styled("Credentials: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&creds, if creds == "None" {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Green)
            }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" j/k: prev/next node   Esc: back to graph ", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let header = Paragraph::new(header_text)
        .block(block(" Node Inspect "))
        .wrap(Wrap { trim: false });
    frame.render_widget(header, chunks[0]);

    // ── Connections ──
    let incoming_str = if incoming.is_empty() {
        Span::styled("  (none — this is an entry point)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(
            format!("  {}", incoming.join(", ")),
            Style::default().fg(Color::White),
        )
    };

    let outgoing_str = if outgoing.is_empty() {
        Span::styled("  (none — this is a terminal node)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(
            format!("  {}", outgoing.join(", ")),
            Style::default().fg(Color::White),
        )
    };

    let conn_text = vec![
        Line::from(vec![
            Span::styled(" ◀ Inputs from: ", Style::default().fg(Color::Cyan).bold()),
            incoming_str,
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ▶ Outputs to:  ", Style::default().fg(Color::Cyan).bold()),
            outgoing_str,
        ]),
    ];

    let conn_widget = Paragraph::new(conn_text)
        .block(block(" Connections "))
        .wrap(Wrap { trim: false });
    frame.render_widget(conn_widget, chunks[1]);

    // ── Parameters (pretty-printed JSON with scroll) ──
    let params_str = if node.parameters.is_null()
        || node.parameters == serde_json::Value::Object(serde_json::Map::new())
    {
        "  (no parameters configured)".to_string()
    } else {
        serde_json::to_string_pretty(&node.parameters).unwrap_or_else(|_| "{}".to_string())
    };

    let line_count = params_str.lines().count() as u16;
    let visible_height = chunks[2].height.saturating_sub(2); // minus borders
    let max_scroll = line_count.saturating_sub(visible_height);
    let scroll = app.node_inspect_scroll.min(max_scroll);

    let scroll_indicator = if line_count > visible_height {
        format!(" Parameters (JSON) — line {}/{} — j/k:scroll g/G:top/bottom ", scroll + 1, line_count)
    } else {
        " Parameters (JSON) ".to_string()
    };

    let params = Paragraph::new(params_str)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(scroll_indicator)
                .title_style(Style::default().fg(Color::Cyan))
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .scroll((scroll, 0))
        .style(Style::default().fg(Color::White));
    frame.render_widget(params, chunks[2]);
}

fn parse_connections(conn: &serde_json::Value) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(obj) = conn.as_object() {
        for (src, outputs) in obj {
            if let Some(main) = outputs.get("main").and_then(|m| m.as_array()) {
                for branch in main {
                    if let Some(targets) = branch.as_array() {
                        for target in targets {
                            if let Some(name) = target.get("node").and_then(|n| n.as_str()) {
                                result.entry(src.clone()).or_default().push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::DarkGray))
}
