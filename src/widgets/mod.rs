pub mod execution_detail;
pub mod execution_list;
pub mod help;
pub mod insights;
pub mod node_detail;
pub mod status_bar;
pub mod tabs;
pub mod trigger_editor;
pub mod workflow_detail;
pub mod workflow_list;
pub mod workflow_node_inspect;

use ratatui::prelude::*;

use crate::app::App;

/// Main render function that dispatches to the appropriate widget.
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(1),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    if app.input_mode == crate::app::InputMode::Trigger {
        // Full-screen trigger editor overlay
        trigger_editor::render(app, frame, area);
    } else {
        tabs::render(app, frame, chunks[0]);

        match &app.view.clone() {
            crate::app::View::WorkflowList => workflow_list::render(app, frame, chunks[1]),
            crate::app::View::WorkflowDetail => workflow_detail::render(app, frame, chunks[1]),
            crate::app::View::WorkflowNodeInspect => {
                workflow_node_inspect::render(app, frame, chunks[1])
            }
            crate::app::View::ExecutionList => execution_list::render(app, frame, chunks[1]),
            crate::app::View::ExecutionDetail => execution_detail::render(app, frame, chunks[1]),
            crate::app::View::NodeDetail(idx) => node_detail::render(app, frame, chunks[1], *idx),
            crate::app::View::Insights | crate::app::View::InsightDetail(_) => {
                insights::render(app, frame, chunks[1])
            }
        }

        status_bar::render(app, frame, chunks[2]);

        if app.show_help {
            help::render_overlay(frame, area);
        }
    }
}
