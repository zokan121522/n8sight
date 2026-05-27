use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{App, InputMode, View};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    // Right-aligned auto-refresh indicator
    let refresh_indicator = if app.auto_refresh {
        let secs_since = (app.now - app.last_refresh).num_seconds();
        let next_in = (app.auto_refresh_interval_secs as i64 - secs_since).max(0);
        format!(" ⟳ {}s ", next_in)
    } else {
        " ⏸ paused ".to_string()
    };

    let refresh_style = if app.auto_refresh {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let content = if app.input_mode == InputMode::Trigger {
        Span::styled(
            " ⚡ Editing webhook payload (full-screen)  —  Ctrl+S to send, Esc to cancel",
            Style::default().fg(Color::Cyan),
        )
    } else if app.input_mode == InputMode::Filter {
        let prefix = match app.view {
            View::WorkflowList => "Filter workflows",
            View::ExecutionList => "Filter executions",
            _ => "Filter",
        };
        Span::styled(
            format!(
                " {} > {}▌  (Enter to apply, Esc to cancel)",
                prefix, app.filter_input
            ),
            Style::default().fg(Color::Yellow),
        )
    } else if app.loading {
        Span::styled(" Loading…", Style::default().fg(Color::Cyan))
    } else if let Some(ref msg) = app.status_message {
        if msg.starts_with("Error") {
            Span::styled(format!(" {}", msg), Style::default().fg(Color::Red))
        } else {
            Span::styled(format!(" {}", msg), Style::default().fg(Color::Green))
        }
    } else {
        let help = match app.view {
            View::WorkflowList => {
                " j/k:nav  Enter:detail  a/i/0:filter  n/s/u:sort  /:search  A:toggle  p:pause  ?:help  q:quit"
            }
            View::WorkflowDetail => {
                " h/j/k/l:pan  Tab:select node  Enter:inspect  t:trigger  p:pause  ?:help  Esc:back"
            }
            View::WorkflowNodeInspect => {
                " j/k:scroll  Ctrl+D/U:page  g/G:top/btm  n/N:next/prev node  t:trigger  Esc:back  ?:help"
            }
            View::ExecutionList => {
                " j/k:nav  Enter:detail  1-5:status  0:clear  n/s/d:sort  /:search  p:pause  ?:help  q:quit"
            }
            View::ExecutionDetail => {
                " j/k:nav  Enter:node detail  R:retry  Esc:back  o:open  x:copy  ?:help"
            }
            View::NodeDetail(_) => " Esc:back  ?:help",
            View::Insights => " j/k:nav  Enter:detail  p:pause  r:refresh  ?:help  q:quit",
            View::InsightDetail(_) => " Esc:back  ?:help",
        };
        Span::styled(help, Style::default().fg(Color::DarkGray))
    };

    // Split area: left for help text, right for refresh indicator
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(refresh_indicator.len() as u16)])
        .split(area);

    let bar = Paragraph::new(Line::from(content)).style(Style::default().bg(Color::Black));
    frame.render_widget(bar, chunks[0]);

    let indicator = Paragraph::new(Line::from(Span::styled(&refresh_indicator, refresh_style)))
        .style(Style::default().bg(Color::Black))
        .alignment(Alignment::Right);
    frame.render_widget(indicator, chunks[1]);
}
