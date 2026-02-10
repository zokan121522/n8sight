use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

pub fn render_overlay(frame: &mut Frame, area: Rect) {
    let overlay_width = 64.min(area.width.saturating_sub(4));
    let overlay_height = 30.min(area.height.saturating_sub(4));
    let x = (area.width - overlay_width) / 2;
    let y = (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

    frame.render_widget(Clear, overlay_area);

    let help_text = vec![
        Line::from(Span::styled(
            "n8sight — Keyboard Reference",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default().fg(Color::Yellow).bold(),
        )),
        Line::from("  j/k, ↑/↓       Move up/down"),
        Line::from("  g/G             Jump to top/bottom"),
        Line::from("  Ctrl+D/U        Half page down/up"),
        Line::from("  PgDn/PgUp       Full page down/up"),
        Line::from("  Enter           Drill into detail"),
        Line::from("  Esc/Backspace   Go back"),
        Line::from("  Tab             Cycle tabs"),
        Line::from("  Alt+1/2/3       Jump to tab"),
        Line::from(""),
        Line::from(Span::styled(
            "Filtering",
            Style::default().fg(Color::Yellow).bold(),
        )),
        Line::from("  /               Start text filter"),
        Line::from("  1-5             Status: err/run/ok/wait/cancel (exec list)"),
        Line::from("  0               Clear filter"),
        Line::from("  a/i             Active/Inactive filter (workflow list)"),
        Line::from(""),
        Line::from(Span::styled(
            "Sorting",
            Style::default().fg(Color::Yellow).bold(),
        )),
        Line::from("  n               Sort by name"),
        Line::from("  s               Sort by status"),
        Line::from("  u               Sort by updated (workflows)"),
        Line::from("  d               Sort by duration (executions)"),
        Line::from(""),
        Line::from(Span::styled(
            "Actions",
            Style::default().fg(Color::Yellow).bold(),
        )),
        Line::from("  r               Refresh data"),
        Line::from("  A               Activate/Deactivate workflow"),
        Line::from("  R               Retry execution"),
        Line::from("  x               Copy URL to clipboard"),
        Line::from("  o               Open in browser"),
        Line::from("  ?               Toggle this help"),
        Line::from("  q (x2)          Quit"),
    ];

    let p = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .title_style(Style::default().fg(Color::Cyan))
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(p, overlay_area);
}
