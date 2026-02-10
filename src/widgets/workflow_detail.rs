use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::collections::HashMap;

use crate::app::App;
use crate::domain::workflow::{WorkflowDetail, WorkflowNode};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let detail = match &app.workflow_detail {
        Some(d) => d,
        None => {
            let p = Paragraph::new("Loading workflow…")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(block(" Workflow Detail "));
            frame.render_widget(p, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Header
            Constraint::Min(1),   // Graph
        ])
        .split(area);

    render_header(detail, frame, chunks[0]);
    render_graph(detail, app, frame, chunks[1]);
}

fn render_header(detail: &WorkflowDetail, frame: &mut Frame, area: Rect) {
    let active_str = if detail.active {
        Span::styled("● Active", Style::default().fg(Color::Green))
    } else {
        Span::styled("○ Inactive", Style::default().fg(Color::DarkGray))
    };

    let tags = if detail.tags.is_empty() {
        "—".to_string()
    } else {
        detail.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Workflow: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&detail.name, Style::default().fg(Color::White).bold()),
            Span::raw("  "),
            active_str,
            Span::raw("  "),
            Span::styled("Nodes: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", detail.nodes.len()), Style::default().fg(Color::Cyan).bold()),
            Span::raw("  "),
            Span::styled("Triggers: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", detail.trigger_nodes().len()), Style::default().fg(Color::Yellow).bold()),
        ]),
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
            Span::raw(&detail.id),
            Span::raw("  "),
            Span::styled("Tags: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&tags, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" h/j/k/l ← ↓ ↑ →: pan   Tab: select node   0: reset   Esc: back ", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let p = Paragraph::new(text)
        .block(block(" Workflow Detail "))
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_graph(detail: &WorkflowDetail, app: &App, frame: &mut Frame, area: Rect) {
    if detail.nodes.is_empty() {
        let p = Paragraph::new("No nodes in this workflow.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(block(" Graph "));
        frame.render_widget(p, area);
        return;
    }

    // Parse connections: source_name -> vec of target_names
    let connections = parse_connections(&detail.connections);

    // Build the canvas: place nodes using their n8n positions, scaled to terminal
    let node_boxes = layout_nodes(&detail.nodes, app.graph_pan_x, app.graph_pan_y);

    // Available drawing area (inside borders)
    let canvas_w = area.width.saturating_sub(2) as i32;
    let canvas_h = area.height.saturating_sub(2) as i32;

    // Render to a vec of Lines
    let mut canvas: Vec<Vec<(char, Style)>> = vec![vec![(' ', Style::default()); canvas_w as usize]; canvas_h as usize];

    // Draw connection lines first (behind nodes)
    for (src_name, targets) in &connections {
        if let Some(src_box) = node_boxes.get(src_name.as_str()) {
            for target_name in targets {
                if let Some(tgt_box) = node_boxes.get(target_name.as_str()) {
                    draw_connection(&mut canvas, src_box, tgt_box, canvas_w, canvas_h);
                }
            }
        }
    }

    // Draw nodes on top
    for (i, node) in detail.nodes.iter().enumerate() {
        if let Some(nb) = node_boxes.get(node.name.as_str()) {
            let is_selected = i == app.graph_selected_node;
            draw_node_box(&mut canvas, nb, node, is_selected, canvas_w, canvas_h);
        }
    }

    // Convert canvas to ratatui Lines
    let lines: Vec<Line> = canvas
        .iter()
        .map(|row| {
            let spans: Vec<Span> = row
                .iter()
                .map(|(ch, style)| Span::styled(ch.to_string(), *style))
                .collect();
            Line::from(spans)
        })
        .collect();

    let graph = Paragraph::new(lines).block(block(" ⬡ Workflow Graph "));
    frame.render_widget(graph, area);
}

// ─── Graph layout helpers ─────────────────────────────────────────────────────

struct NodeBox {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// Scale n8n canvas positions to terminal coordinates.
fn layout_nodes(nodes: &[WorkflowNode], pan_x: i32, pan_y: i32) -> HashMap<&str, NodeBox> {
    let mut result = HashMap::new();

    if nodes.is_empty() {
        return result;
    }

    // Find bounds of all node positions
    let xs: Vec<f64> = nodes.iter().filter_map(|n| n.position.first().copied()).collect();
    let ys: Vec<f64> = nodes.iter().filter_map(|n| n.position.get(1).copied()).collect();

    if xs.is_empty() || ys.is_empty() {
        // No positions: lay out in a horizontal line
        for (i, node) in nodes.iter().enumerate() {
            let box_w = (node.name.len() as i32 + 4).max(16).min(24);
            result.insert(
                node.name.as_str(),
                NodeBox {
                    x: (i as i32) * 28 - pan_x,
                    y: 2 - pan_y,
                    w: box_w,
                    h: 3,
                },
            );
        }
        return result;
    }

    let min_x = xs.iter().cloned().fold(f64::MAX, f64::min);
    let max_x = xs.iter().cloned().fold(f64::MIN, f64::max);
    let min_y = ys.iter().cloned().fold(f64::MAX, f64::min);
    let max_y = ys.iter().cloned().fold(f64::MIN, f64::max);

    let range_x = (max_x - min_x).max(1.0);
    let range_y = (max_y - min_y).max(1.0);

    // Scale factor: map n8n positions to a ~200 col x ~60 row virtual canvas
    // (pan shifts the viewport)
    let scale_x = 160.0 / range_x;
    let scale_y = 40.0 / range_y;

    for node in nodes {
        let nx = node.position.first().copied().unwrap_or(0.0);
        let ny = node.position.get(1).copied().unwrap_or(0.0);

        let box_w = (node.name.len() as i32 + 4).max(16).min(24);
        let tx = ((nx - min_x) * scale_x) as i32 + 2 - pan_x;
        let ty = ((ny - min_y) * scale_y) as i32 + 1 - pan_y;

        result.insert(
            node.name.as_str(),
            NodeBox {
                x: tx,
                y: ty,
                w: box_w,
                h: 3,
            },
        );
    }

    result
}

/// Draw a node box on the canvas.
fn draw_node_box(
    canvas: &mut [Vec<(char, Style)>],
    nb: &NodeBox,
    node: &WorkflowNode,
    selected: bool,
    cw: i32,
    ch: i32,
) {
    let is_trigger = node.node_type.contains("trigger")
        || node.node_type.contains("webhook")
        || node.node_type.contains("cron")
        || node.node_type.contains("schedule");

    let border_style = if selected {
        Style::default().fg(Color::Cyan).bold()
    } else if node.disabled {
        Style::default().fg(Color::DarkGray)
    } else if is_trigger {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let name_style = if selected {
        Style::default().fg(Color::Cyan).bold()
    } else if node.disabled {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let short_type = WorkflowDetail::short_node_type(&node.node_type);
    let type_style = Style::default().fg(Color::DarkGray);

    // Top border: ┌──────────┐
    for dx in 0..nb.w {
        let ch_c = if dx == 0 { '┌' } else if dx == nb.w - 1 { '┐' } else { '─' };
        put(canvas, nb.x + dx, nb.y, ch_c, border_style, cw, ch);
    }

    // Middle: │ Name     │
    put(canvas, nb.x, nb.y + 1, '│', border_style, cw, ch);
    let display_name = truncate(&node.name, (nb.w - 3) as usize);
    for (dx, c) in display_name.chars().enumerate() {
        put(canvas, nb.x + 1 + dx as i32, nb.y + 1, c, name_style, cw, ch);
    }
    put(canvas, nb.x + nb.w - 1, nb.y + 1, '│', border_style, cw, ch);

    // Type line: │ webhook  │
    put(canvas, nb.x, nb.y + 2, '│', border_style, cw, ch);
    let display_type = truncate(&short_type, (nb.w - 3) as usize);
    for (dx, c) in display_type.chars().enumerate() {
        put(canvas, nb.x + 1 + dx as i32, nb.y + 2, c, type_style, cw, ch);
    }
    put(canvas, nb.x + nb.w - 1, nb.y + 2, '│', border_style, cw, ch);

    // Bottom border: └──────────┘
    for dx in 0..nb.w {
        let ch_c = if dx == 0 { '└' } else if dx == nb.w - 1 { '┘' } else { '─' };
        put(canvas, nb.x + dx, nb.y + 3, ch_c, border_style, cw, ch);
    }
}

/// Draw a connection line between two node boxes.
fn draw_connection(
    canvas: &mut [Vec<(char, Style)>],
    src: &NodeBox,
    tgt: &NodeBox,
    cw: i32,
    ch: i32,
) {
    let style = Style::default().fg(Color::DarkGray);

    // Start from right edge of source, end at left edge of target
    let sx = src.x + src.w;
    let sy = src.y + src.h / 2;
    let tx = tgt.x - 1;
    let ty = tgt.y + tgt.h / 2;

    if sx >= tx {
        // Nodes overlap horizontally, just draw an arrow
        put(canvas, tx, ty, '◀', Style::default().fg(Color::Cyan), cw, ch);
        return;
    }

    // Horizontal line from source
    let mid_x = (sx + tx) / 2;

    for x in sx..=mid_x {
        put(canvas, x, sy, '─', style, cw, ch);
    }

    // Vertical segment
    if sy != ty {
        let (y_start, y_end) = if sy < ty { (sy, ty) } else { (ty, sy) };
        for y in y_start..=y_end {
            put(canvas, mid_x, y, '│', style, cw, ch);
        }
        // Corners
        if sy < ty {
            put(canvas, mid_x, sy, '┐', style, cw, ch);
            put(canvas, mid_x, ty, '└', style, cw, ch);
        } else {
            put(canvas, mid_x, sy, '┘', style, cw, ch);
            put(canvas, mid_x, ty, '┌', style, cw, ch);
        }
    }

    // Horizontal line to target
    for x in mid_x..tx {
        put(canvas, x, ty, '─', style, cw, ch);
    }

    // Arrow head
    put(canvas, tx, ty, '▶', Style::default().fg(Color::Cyan), cw, ch);
}

/// Parse n8n connections JSON into a map of source_name -> vec<target_name>.
fn parse_connections(conn: &serde_json::Value) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    if let Some(obj) = conn.as_object() {
        for (src_name, outputs) in obj {
            if let Some(main) = outputs.get("main").and_then(|m| m.as_array()) {
                for branch in main {
                    if let Some(targets) = branch.as_array() {
                        for target in targets {
                            if let Some(node_name) = target.get("node").and_then(|n| n.as_str()) {
                                result
                                    .entry(src_name.clone())
                                    .or_default()
                                    .push(node_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

fn put(
    canvas: &mut [Vec<(char, Style)>],
    x: i32,
    y: i32,
    ch: char,
    style: Style,
    cw: i32,
    canvas_h: i32,
) {
    if x >= 0 && y >= 0 && x < cw && y < canvas_h {
        canvas[y as usize][x as usize] = (ch, style);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::DarkGray))
}
