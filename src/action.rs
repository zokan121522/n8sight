use crossterm::event::KeyEvent;

use crate::domain::execution::{ExecutionDetail, ExecutionSummary};
use crate::domain::insights::InsightsResult;
use crate::domain::workflow::{WorkflowDetail, WorkflowSummary};

/// All possible actions in the TUI state machine.
#[derive(Debug, Clone)]
pub enum Action {
    /// Raw key event from the terminal — dispatched by App based on current mode/view.
    Key(KeyEvent),

    // -- Navigation --
    Quit,
    Back,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    PageUp,
    PageDown,
    Select,
    CycleTab,
    GoToTab(usize),

    // -- Filtering --
    StartFilter,
    FilterChar(char),
    FilterBackspace,
    CancelFilter,
    ApplyFilter,
    StatusFilter(Option<String>),
    ActiveFilter(Option<bool>),

    // -- Sorting --
    SortByName,
    SortByStatus,
    SortByUpdated,
    SortByDuration,

    // -- Actions --
    Refresh,
    ToggleActivation,
    RetryExecution,
    CopyUrl,
    OpenInBrowser,
    ToggleHelp,

    // -- Data loaded (responses from the async worker) --
    WorkflowsLoaded(Vec<WorkflowSummary>),
    WorkflowDetailLoaded(Box<WorkflowDetail>),
    ExecutionsLoaded(Vec<ExecutionSummary>),
    ExecutionDetailLoaded(Box<ExecutionDetail>),
    InsightsLoaded(Box<InsightsResult>),
    LoadError(String),

    // -- Internal --
    Tick,
}

/// Side effects returned from App::update(). Processed by the main loop.
#[derive(Debug)]
pub enum Effect {
    SendWorkerRequest(crate::cli_worker::WorkerRequest),
    CopyToClipboard(String),
    OpenUrl(String),
    Quit,
}
