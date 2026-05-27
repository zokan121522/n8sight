use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::text::Line;

use crate::app::App;

/// Full-screen editor for the webhook trigger JSON payload.
///
/// Rendered as an overlay in the main content area when `InputMode::Trigger` is active.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    // ── Outer block with title ─────────────────────────────────────────
    let webhook_path = app
        .trigger_webhook_path
        .as_deref()
        .unwrap_or("(unknown)");

    let outer = Block::default()
        .title(format!(" ⚡ Trigger Webhook "))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // ── Split inner area: top info, middle editor, bottom help ─────────
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Webhook path
            Constraint::Min(1),      // Editor area
            Constraint::Length(1),   // Help bar
        ])
        .split(inner);

    // ── Webhook path line ──────────────────────────────────────────────
    let path_style = Style::default().fg(Color::DarkGray);
    let path_line = Line::from(vec![
        Span::styled(" Webhook: ", Style::default().fg(Color::Gray)),
        Span::styled(webhook_path, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);
    let path_widget = Paragraph::new(path_line).style(path_style);
    frame.render_widget(path_widget, chunks[0]);

    // ── Editor area ────────────────────────────────────────────────────
    let editor_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White).dim());

    let editor_inner = editor_block.inner(chunks[1]);

    // Calculate visible lines for scroll
    let editor_height = editor_inner.height.max(1) as usize;
    let editor_width = editor_inner.width.max(1) as usize;

    // Build the display text: each line gets a gutter with line number
    let lines: Vec<String> = app
        .trigger_input
        .lines()
        .map(|l| l.to_string())
        .collect();

    let total_lines = lines.len().max(1);
    let line_num_width = total_lines.to_string().len().max(2);

    // Scroll offset: try to keep cursor visible
    // We approximate "cursor line" by counting newlines before cursor position
    let cursor_line = app.trigger_input[..app.trigger_input.len()]
        .chars()
        .filter(|&c| c == '\n')
        .count();

    let scroll_offset = if cursor_line >= editor_height {
        cursor_line - editor_height + 1
    } else {
        0
    };

    // Render visible lines
    let visible_lines: Vec<Line> = lines
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(editor_height)
        .map(|(i, text)| {
            let line_num = i + 1;
            let is_current_line = i == cursor_line;
            let num_style = if is_current_line {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let text_style = if is_current_line {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(vec![
                Span::styled(
                    format!("{:>width$} ", line_num, width = line_num_width),
                    num_style,
                ),
                Span::styled(text.clone(), text_style),
            ])
        })
        .collect::<Vec<_>>();

    // Fill remaining space with empty lines
    let empty_lines = editor_height.saturating_sub(visible_lines.len());
    let mut all_lines = visible_lines;
    for _ in 0..empty_lines {
        all_lines.push(Line::from(vec![
            Span::styled(
                format!("{:>width$} ", "", width = line_num_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("~"),
        ]));
    }

    let editor_widget = Paragraph::new(all_lines)
        .block(editor_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(editor_widget, chunks[1]);

    // ── Cursor visual: render a simulated cursor block ─────────────────
    // We use the cursor position to show where text is being typed
    let cursor_col = if let Some(pos) = app.trigger_input.chars().rev().position(|c| c == '\n') {
        pos
    } else {
        app.trigger_input.len()
    };
    let cursor_col = cursor_col.min(editor_width.saturating_sub(line_num_width + 2));

    // Place the cursor at the end of the text in the editor area
    // The visual line offset within the visible window
    let visual_line = cursor_line.saturating_sub(scroll_offset);
    if visual_line < editor_height {
        let cursor_x = (line_num_width + 1 + cursor_col) as u16;
        let cursor_y = (1 + visual_line) as u16; // +1 for border
        frame.set_cursor_position(Position::new(
            editor_inner.x + cursor_x,
            editor_inner.y + cursor_y,
        ));
    }

    // ── Bottom help bar ─────────────────────────────────────────────────
    let help_style = Style::default().fg(Color::DarkGray).bg(Color::Black);
    let help_text = Line::from(vec![
        Span::styled(" Ctrl+S", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" send  "),
        Span::styled("Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw(" cancel  "),
        Span::styled("Ctrl+U", Style::default().fg(Color::Yellow)),
        Span::raw(" clear line  "),
        Span::styled("Ctrl+W", Style::default().fg(Color::Yellow)),
        Span::raw(" del word"),
        Span::raw(format!(
            "  │  Ln {} Col {}",
            cursor_line + 1,
            cursor_col + 1,
        )),
    ]);
    let help_widget = Paragraph::new(help_text).style(help_style);
    frame.render_widget(help_widget, chunks[2]);
}
