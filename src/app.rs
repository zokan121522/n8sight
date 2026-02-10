use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, Effect};
use crate::cli_worker::WorkerRequest;
use crate::client::{ExecutionFilter, WorkflowFilter};
use crate::domain::execution::{ExecutionDetail, ExecutionSummary, NodeRunResult, StatusCounts};
use crate::domain::insights::InsightsResult;
use crate::domain::workflow::{WorkflowDetail, WorkflowSummary};
use crate::scroll_state::ScrollState;

// ─── View / Mode enums ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    WorkflowList,
    WorkflowDetail,
    WorkflowNodeInspect, // Expanded view of a single node in the graph
    ExecutionList,
    ExecutionDetail,
    NodeDetail(usize),
    Insights,
    InsightDetail(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
}

// ─── App state ───────────────────────────────────────────────────────────────

pub struct App {
    // -- Chrome --
    pub view: View,
    pub input_mode: InputMode,
    pub show_help: bool,
    pub quit_requested: bool,
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub loading: bool,
    pub active_tab: usize, // 0=Workflows, 1=Executions, 2=Insights
    pub base_url: String,  // Full n8n instance URL for generating links

    // -- Auto-refresh --
    pub auto_refresh: bool,
    pub auto_refresh_interval_secs: u64,
    pub last_refresh: DateTime<Utc>,

    // -- Workflow list --
    pub workflows: Vec<WorkflowSummary>,
    pub wf_scroll: ScrollState,
    pub wf_filter_text: String,
    pub wf_active_filter: Option<bool>,
    pub wf_sort: SortKind,
    cached_filtered_wf: Option<Vec<WorkflowSummary>>,
    wf_cache_dirty: bool,

    // -- Workflow detail --
    pub workflow_detail: Option<WorkflowDetail>,
    pub graph_pan_x: i32,
    pub graph_pan_y: i32,
    pub graph_selected_node: usize,
    pub node_inspect_scroll: u16,

    // -- Execution list --
    pub executions: Vec<ExecutionSummary>,
    pub exec_scroll: ScrollState,
    pub exec_filter_text: String,
    pub exec_status_filter: Option<String>,
    pub exec_sort: SortKind,
    pub exec_counts: StatusCounts,
    cached_filtered_exec: Option<Vec<ExecutionSummary>>,
    exec_cache_dirty: bool,

    // -- Execution detail --
    pub execution_detail: Option<ExecutionDetail>,
    pub node_scroll: ScrollState,
    /// Cached parsed node runs (avoid re-parsing JSON every frame).
    pub cached_node_runs: Vec<NodeRunResult>,

    // -- Insights --
    pub insights_result: Option<InsightsResult>,
    pub insight_scroll: ScrollState,

    // -- Filter input --
    pub filter_input: String,

    // -- Render context: captured once per frame --
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SortKind {
    #[default]
    None,
    NameAsc,
    NameDesc,
    StatusAsc,
    StatusDesc,
    UpdatedAsc,
    UpdatedDesc,
    DurationAsc,
    DurationDesc,
}

// ─── Constructor / Init ──────────────────────────────────────────────────────

impl App {
    pub fn new(base_url: String) -> Self {
        Self {
            view: View::WorkflowList,
            input_mode: InputMode::Normal,
            show_help: false,
            quit_requested: false,
            should_quit: false,
            status_message: None,
            loading: true,
            active_tab: 0,
            base_url,

            auto_refresh: true,
            auto_refresh_interval_secs: 15,
            last_refresh: Utc::now(),

            workflows: Vec::new(),
            wf_scroll: ScrollState::new(),
            wf_filter_text: String::new(),
            wf_active_filter: None,
            wf_sort: SortKind::None,
            cached_filtered_wf: None,
            wf_cache_dirty: true,

            workflow_detail: None,
            graph_pan_x: 0,
            graph_pan_y: 0,
            graph_selected_node: 0,
            node_inspect_scroll: 0,

            executions: Vec::new(),
            exec_scroll: ScrollState::new(),
            exec_filter_text: String::new(),
            exec_status_filter: None,
            exec_sort: SortKind::None,
            exec_counts: StatusCounts::default(),
            cached_filtered_exec: None,
            exec_cache_dirty: true,

            execution_detail: None,
            node_scroll: ScrollState::new(),
            cached_node_runs: Vec::new(),

            insights_result: None,
            insight_scroll: ScrollState::new(),

            filter_input: String::new(),
            now: Utc::now(),
        }
    }

    /// Effects to run on startup.
    pub fn init_effects() -> Vec<Effect> {
        vec![
            Effect::SendWorkerRequest(WorkerRequest::FetchWorkflows(WorkflowFilter::default())),
            Effect::SendWorkerRequest(WorkerRequest::FetchExecutions(ExecutionFilter::default())),
        ]
    }

    /// Call once per frame before rendering to capture a consistent "now".
    pub fn tick_frame(&mut self) {
        self.now = Utc::now();
    }

    // ─── Pure update: returns effects, no side effects ───────────────────────

    pub fn update(&mut self, action: Action) -> Vec<Effect> {
        // Clear transient status on any real action
        if !matches!(action, Action::Tick) {
            self.status_message = None;
        }

        match action {
            Action::Key(key) => self.handle_key(key),
            Action::Quit => {
                self.should_quit = true;
                vec![Effect::Quit]
            }
            Action::Tick => self.handle_tick(),

            // ── Navigation (produced by handle_key) ──
            Action::Back => {
                self.quit_requested = false;
                self.handle_back();
                vec![]
            }
            Action::MoveUp => {
                self.quit_requested = false;
                self.active_scroll_mut().move_up();
                vec![]
            }
            Action::MoveDown => {
                self.quit_requested = false;
                self.active_scroll_mut().move_down();
                vec![]
            }
            Action::MoveTop => {
                self.active_scroll_mut().move_top();
                vec![]
            }
            Action::MoveBottom => {
                self.active_scroll_mut().move_bottom();
                vec![]
            }
            Action::PageUp => {
                self.active_scroll_mut().page_up(20);
                vec![]
            }
            Action::PageDown => {
                self.active_scroll_mut().page_down(20);
                vec![]
            }
            Action::Select => self.handle_select(),
            Action::CycleTab => {
                self.quit_requested = false;
                self.active_tab = (self.active_tab + 1) % 3;
                self.switch_to_tab()
            }
            Action::GoToTab(idx) => {
                self.quit_requested = false;
                self.active_tab = idx.min(2);
                self.switch_to_tab()
            }

            // ── Filtering ──
            Action::StartFilter => {
                self.input_mode = InputMode::Filter;
                self.filter_input.clear();
                vec![]
            }
            Action::FilterChar(c) => {
                self.filter_input.push(c);
                self.invalidate_filter_cache();
                vec![]
            }
            Action::FilterBackspace => {
                self.filter_input.pop();
                self.invalidate_filter_cache();
                vec![]
            }
            Action::CancelFilter => {
                self.input_mode = InputMode::Normal;
                self.filter_input.clear();
                // Don't apply - revert to previous filter
                self.invalidate_filter_cache();
                vec![]
            }
            Action::ApplyFilter => {
                self.input_mode = InputMode::Normal;
                match self.view {
                    View::WorkflowList => {
                        self.wf_filter_text = self.filter_input.clone();
                        self.wf_scroll.reset();
                    }
                    View::ExecutionList => {
                        self.exec_filter_text = self.filter_input.clone();
                        self.exec_scroll.reset();
                    }
                    _ => {}
                }
                self.filter_input.clear();
                self.invalidate_filter_cache();
                vec![]
            }
            Action::StatusFilter(status) => {
                self.exec_status_filter = status;
                self.exec_scroll.reset();
                self.exec_cache_dirty = true;
                vec![]
            }
            Action::ActiveFilter(active) => {
                if self.wf_active_filter == active {
                    self.wf_active_filter = None;
                } else {
                    self.wf_active_filter = active;
                }
                self.wf_scroll.reset();
                self.wf_cache_dirty = true;
                vec![]
            }

            // ── Sorting ──
            Action::SortByName => {
                self.toggle_sort_name();
                vec![]
            }
            Action::SortByStatus => {
                self.toggle_sort_status();
                vec![]
            }
            Action::SortByUpdated => {
                self.toggle_sort_updated();
                vec![]
            }
            Action::SortByDuration => {
                self.toggle_sort_duration();
                vec![]
            }

            // ── Actions ──
            Action::Refresh => self.handle_refresh(),
            Action::ToggleActivation => self.handle_toggle_activation(),
            Action::RetryExecution => self.handle_retry(),
            Action::CopyUrl => {
                if let Some(url) = self.current_item_url() {
                    self.status_message = Some(format!("Copied: {}", url));
                    vec![Effect::CopyToClipboard(url)]
                } else {
                    vec![]
                }
            }
            Action::OpenInBrowser => {
                if let Some(url) = self.current_item_url() {
                    self.status_message = Some(format!("Opened: {}", url));
                    vec![Effect::OpenUrl(url)]
                } else {
                    vec![]
                }
            }
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                vec![]
            }

            // ── Data loaded ──
            Action::WorkflowsLoaded(workflows) => {
                self.workflows = workflows;
                self.loading = false;
                self.last_refresh = self.now;
                self.wf_cache_dirty = true;
                let filtered = self.filtered_workflows();
                self.wf_scroll.set_len(filtered.len());
                vec![]
            }
            Action::WorkflowDetailLoaded(detail) => {
                self.workflow_detail = Some(*detail);
                self.view = View::WorkflowDetail;
                self.graph_pan_x = 0;
                self.graph_pan_y = 0;
                self.graph_selected_node = 0;
                self.loading = false;
                vec![]
            }
            Action::ExecutionsLoaded(executions) => {
                self.exec_counts = StatusCounts::from_executions(&executions);
                self.executions = executions;
                self.loading = false;
                self.last_refresh = self.now;
                self.exec_cache_dirty = true;
                let filtered = self.filtered_executions();
                self.exec_scroll.set_len(filtered.len());
                vec![]
            }
            Action::ExecutionDetailLoaded(detail) => {
                // Cache node runs immediately — never reparse from JSON again
                self.cached_node_runs = detail.node_runs();
                self.node_scroll.set_len(self.cached_node_runs.len());
                self.node_scroll.reset();
                self.execution_detail = Some(*detail);
                self.view = View::ExecutionDetail;
                self.loading = false;
                vec![]
            }
            Action::InsightsLoaded(result) => {
                self.insight_scroll.set_len(result.findings.len());
                self.insights_result = Some(*result);
                self.loading = false;
                vec![]
            }
            Action::LoadError(msg) => {
                self.status_message = Some(format!("Error: {}", msg));
                self.loading = false;
                vec![]
            }
        }
    }

    // ─── Key mapping: mode + view aware ──────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match self.input_mode {
            InputMode::Filter => self.handle_filter_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match key.code {
            KeyCode::Esc => self.update(Action::CancelFilter),
            KeyCode::Enter => self.update(Action::ApplyFilter),
            KeyCode::Backspace => self.update(Action::FilterBackspace),
            KeyCode::Char(c) => self.update(Action::FilterChar(c)),
            _ => vec![],
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        // Graph panning in WorkflowDetail — intercept arrow keys before universal nav
        if self.view == View::WorkflowDetail {
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => { self.graph_pan_x -= 4; return vec![]; }
                KeyCode::Right | KeyCode::Char('l') => { self.graph_pan_x += 4; return vec![]; }
                KeyCode::Up | KeyCode::Char('k') => { self.graph_pan_y -= 2; return vec![]; }
                KeyCode::Down | KeyCode::Char('j') => { self.graph_pan_y += 2; return vec![]; }
                KeyCode::Tab => {
                    if let Some(ref d) = self.workflow_detail {
                        if !d.nodes.is_empty() {
                            self.graph_selected_node = (self.graph_selected_node + 1) % d.nodes.len();
                        }
                    }
                    return vec![];
                }
                KeyCode::Enter => {
                    self.view = View::WorkflowNodeInspect;
                    return vec![];
                }
                KeyCode::Char('0') => {
                    self.graph_pan_x = 0;
                    self.graph_pan_y = 0;
                    return vec![];
                }
                _ => {} // fall through to universal keys
            }
        }

        // WorkflowNodeInspect: j/k scroll params, Tab/n/N cycle nodes
        if self.view == View::WorkflowNodeInspect {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.node_inspect_scroll = self.node_inspect_scroll.saturating_add(1);
                    return vec![];
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.node_inspect_scroll = self.node_inspect_scroll.saturating_sub(1);
                    return vec![];
                }
                KeyCode::Char('d') => {
                    self.node_inspect_scroll = self.node_inspect_scroll.saturating_add(10);
                    return vec![];
                }
                KeyCode::Char('u') => {
                    self.node_inspect_scroll = self.node_inspect_scroll.saturating_sub(10);
                    return vec![];
                }
                KeyCode::Char('g') => {
                    self.node_inspect_scroll = 0;
                    return vec![];
                }
                KeyCode::Char('G') => {
                    self.node_inspect_scroll = u16::MAX; // will be clamped by renderer
                    return vec![];
                }
                KeyCode::Tab | KeyCode::Char('n') => {
                    if let Some(ref d) = self.workflow_detail {
                        if !d.nodes.is_empty() {
                            self.graph_selected_node = (self.graph_selected_node + 1) % d.nodes.len();
                            self.node_inspect_scroll = 0;
                        }
                    }
                    return vec![];
                }
                KeyCode::Char('N') | KeyCode::BackTab => {
                    if let Some(ref d) = self.workflow_detail {
                        if !d.nodes.is_empty() {
                            self.graph_selected_node = if self.graph_selected_node == 0 {
                                d.nodes.len() - 1
                            } else {
                                self.graph_selected_node - 1
                            };
                            self.node_inspect_scroll = 0;
                        }
                    }
                    return vec![];
                }
                _ => {} // fall through
            }
        }

        let action = match key.code {
            // Universal navigation
            KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
            KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
            KeyCode::Char('g') if !key.modifiers.contains(KeyModifiers::SHIFT) => Action::MoveTop,
            KeyCode::Char('G') => Action::MoveBottom,
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::PageDown,
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::PageUp,
            KeyCode::PageDown => Action::PageDown,
            KeyCode::PageUp => Action::PageUp,
            KeyCode::Enter => Action::Select,
            KeyCode::Esc | KeyCode::Backspace => Action::Back,
            KeyCode::Tab => Action::CycleTab,

            // Direct tab jumps
            KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::ALT) => Action::GoToTab(0),
            KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::ALT) => Action::GoToTab(1),
            KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT) => Action::GoToTab(2),

            // View-contextual keys
            _ => return self.handle_view_key(key),
        };
        self.update(action)
    }

    /// Keys that mean different things depending on which view is active.
    fn handle_view_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        let action = match (&self.view, key.code) {
            // ── Workflow list keys ──
            (View::WorkflowList, KeyCode::Char('a')) => Action::ActiveFilter(Some(true)),
            (View::WorkflowList, KeyCode::Char('i')) => Action::ActiveFilter(Some(false)),
            (View::WorkflowList, KeyCode::Char('0')) => Action::ActiveFilter(None),
            (View::WorkflowList, KeyCode::Char('A')) => Action::ToggleActivation,
            (View::WorkflowList, KeyCode::Char('n')) => Action::SortByName,
            (View::WorkflowList, KeyCode::Char('s')) => Action::SortByStatus,
            (View::WorkflowList, KeyCode::Char('u')) => Action::SortByUpdated,

            // ── Execution list keys ──
            (View::ExecutionList, KeyCode::Char('1')) => Action::StatusFilter(Some("error".into())),
            (View::ExecutionList, KeyCode::Char('2')) => {
                Action::StatusFilter(Some("running".into()))
            }
            (View::ExecutionList, KeyCode::Char('3')) => {
                Action::StatusFilter(Some("success".into()))
            }
            (View::ExecutionList, KeyCode::Char('4')) => {
                Action::StatusFilter(Some("waiting".into()))
            }
            (View::ExecutionList, KeyCode::Char('5')) => {
                Action::StatusFilter(Some("canceled".into()))
            }
            (View::ExecutionList, KeyCode::Char('0')) => Action::StatusFilter(None),
            (View::ExecutionList, KeyCode::Char('s')) => Action::SortByStatus,
            (View::ExecutionList, KeyCode::Char('n')) => Action::SortByName,
            (View::ExecutionList, KeyCode::Char('d')) => Action::SortByDuration,

            // ── Execution detail keys ──
            (View::ExecutionDetail, KeyCode::Char('R')) => Action::RetryExecution,

            // ── Universal action keys ──
            (_, KeyCode::Char('/')) => Action::StartFilter,
            (_, KeyCode::Char('r')) => Action::Refresh,
            (_, KeyCode::Char('p')) => {
                self.auto_refresh = !self.auto_refresh;
                self.status_message = Some(if self.auto_refresh {
                    format!("Auto-refresh resumed (every {}s)", self.auto_refresh_interval_secs)
                } else {
                    "Auto-refresh paused".into()
                });
                return vec![];
            }
            (_, KeyCode::Char('x')) => Action::CopyUrl,
            (_, KeyCode::Char('o')) => Action::OpenInBrowser,
            (_, KeyCode::Char('?')) => Action::ToggleHelp,
            (_, KeyCode::Char('q')) => {
                if self.quit_requested {
                    Action::Quit
                } else {
                    self.quit_requested = true;
                    self.status_message = Some("Press q again to quit".into());
                    return vec![];
                }
            }

            _ => return vec![],
        };
        self.update(action)
    }

    // ─── Action handlers ─────────────────────────────────────────────────────

    fn handle_tick(&mut self) -> Vec<Effect> {
        if !self.auto_refresh || self.loading {
            return vec![];
        }

        let elapsed = (self.now - self.last_refresh).num_seconds();
        if elapsed < self.auto_refresh_interval_secs as i64 {
            return vec![];
        }

        // Time to auto-refresh — only refresh the data for the current view
        self.last_refresh = self.now;
        match self.view {
            View::WorkflowList => {
                vec![Effect::SendWorkerRequest(WorkerRequest::FetchWorkflows(
                    WorkflowFilter {
                        active: self.wf_active_filter,
                        ..Default::default()
                    },
                ))]
            }
            View::ExecutionList => {
                vec![Effect::SendWorkerRequest(WorkerRequest::FetchExecutions(
                    ExecutionFilter {
                        status: self.exec_status_filter.clone(),
                        ..Default::default()
                    },
                ))]
            }
            View::Insights => {
                vec![Effect::SendWorkerRequest(WorkerRequest::RunInsights(5))]
            }
            // Don't auto-refresh detail views (would be disorienting)
            _ => vec![],
        }
    }

    fn handle_back(&mut self) {
        if self.show_help {
            self.show_help = false;
        } else {
            match self.view {
                View::WorkflowDetail => self.view = View::WorkflowList,
                View::WorkflowNodeInspect => self.view = View::WorkflowDetail,
                View::ExecutionDetail => self.view = View::ExecutionList,
                View::NodeDetail(_) => self.view = View::ExecutionDetail,
                View::InsightDetail(_) => self.view = View::Insights,
                _ => {}
            }
        }
    }

    fn handle_select(&mut self) -> Vec<Effect> {
        match self.view {
            View::WorkflowList => {
                let filtered = self.filtered_workflows();
                if let Some(wf) = filtered.get(self.wf_scroll.selected()) {
                    let id = wf.id.clone();
                    self.loading = true;
                    return vec![Effect::SendWorkerRequest(
                        WorkerRequest::FetchWorkflowDetail(id),
                    )];
                }
            }
            View::ExecutionList => {
                let filtered = self.filtered_executions();
                if let Some(exec) = filtered.get(self.exec_scroll.selected()) {
                    let id = exec.id.clone();
                    self.loading = true;
                    return vec![Effect::SendWorkerRequest(
                        WorkerRequest::FetchExecutionDetail(id, true),
                    )];
                }
            }
            View::ExecutionDetail => {
                self.view = View::NodeDetail(self.node_scroll.selected());
            }
            View::Insights => {
                self.view = View::InsightDetail(self.insight_scroll.selected());
            }
            _ => {}
        }
        vec![]
    }

    fn switch_to_tab(&mut self) -> Vec<Effect> {
        match self.active_tab {
            0 => {
                self.view = View::WorkflowList;
                vec![]
            }
            1 => {
                self.view = View::ExecutionList;
                vec![]
            }
            2 => {
                self.view = View::Insights;
                if self.insights_result.is_none() && !self.loading {
                    self.loading = true;
                    vec![Effect::SendWorkerRequest(WorkerRequest::RunInsights(5))]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    fn handle_refresh(&mut self) -> Vec<Effect> {
        self.loading = true;
        match self.view {
            View::WorkflowList | View::WorkflowDetail => {
                vec![Effect::SendWorkerRequest(WorkerRequest::FetchWorkflows(
                    WorkflowFilter {
                        active: self.wf_active_filter,
                        ..Default::default()
                    },
                ))]
            }
            View::ExecutionList | View::ExecutionDetail => {
                vec![Effect::SendWorkerRequest(WorkerRequest::FetchExecutions(
                    ExecutionFilter {
                        status: self.exec_status_filter.clone(),
                        ..Default::default()
                    },
                ))]
            }
            View::Insights => {
                vec![Effect::SendWorkerRequest(WorkerRequest::RunInsights(5))]
            }
            _ => {
                self.loading = false;
                vec![]
            }
        }
    }

    fn handle_toggle_activation(&mut self) -> Vec<Effect> {
        if self.view != View::WorkflowList {
            return vec![];
        }
        let filtered = self.filtered_workflows();
        if let Some(wf) = filtered.get(self.wf_scroll.selected()) {
            let id = wf.id.clone();
            self.loading = true;
            if wf.active {
                vec![Effect::SendWorkerRequest(
                    WorkerRequest::DeactivateWorkflow(id),
                )]
            } else {
                vec![Effect::SendWorkerRequest(WorkerRequest::ActivateWorkflow(
                    id,
                ))]
            }
        } else {
            vec![]
        }
    }

    fn handle_retry(&mut self) -> Vec<Effect> {
        if self.view != View::ExecutionDetail {
            return vec![];
        }
        if let Some(ref detail) = self.execution_detail {
            let id = detail.id.clone();
            self.loading = true;
            vec![Effect::SendWorkerRequest(WorkerRequest::RetryExecution(id))]
        } else {
            vec![]
        }
    }

    // ─── Sorting ─────────────────────────────────────────────────────────────

    fn toggle_sort_name(&mut self) {
        let sort = match self.view {
            View::WorkflowList => &mut self.wf_sort,
            View::ExecutionList => &mut self.exec_sort,
            _ => return,
        };
        *sort = match sort {
            SortKind::NameAsc => SortKind::NameDesc,
            _ => SortKind::NameAsc,
        };
        self.invalidate_filter_cache();
    }

    fn toggle_sort_status(&mut self) {
        let sort = match self.view {
            View::WorkflowList => &mut self.wf_sort,
            View::ExecutionList => &mut self.exec_sort,
            _ => return,
        };
        *sort = match sort {
            SortKind::StatusAsc => SortKind::StatusDesc,
            _ => SortKind::StatusAsc,
        };
        self.invalidate_filter_cache();
    }

    fn toggle_sort_updated(&mut self) {
        let sort = match self.view {
            View::WorkflowList => &mut self.wf_sort,
            _ => return,
        };
        *sort = match sort {
            SortKind::UpdatedAsc => SortKind::UpdatedDesc,
            _ => SortKind::UpdatedAsc,
        };
        self.invalidate_filter_cache();
    }

    fn toggle_sort_duration(&mut self) {
        if self.view != View::ExecutionList {
            return;
        }
        self.exec_sort = match self.exec_sort {
            SortKind::DurationAsc => SortKind::DurationDesc,
            _ => SortKind::DurationAsc,
        };
        self.invalidate_filter_cache();
    }

    // ─── Cached filtered + sorted accessors ──────────────────────────────────

    fn invalidate_filter_cache(&mut self) {
        self.wf_cache_dirty = true;
        self.exec_cache_dirty = true;
    }

    pub fn filtered_workflows(&mut self) -> Vec<WorkflowSummary> {
        if !self.wf_cache_dirty {
            if let Some(ref cached) = self.cached_filtered_wf {
                return cached.clone();
            }
        }

        let filter_text =
            if self.input_mode == InputMode::Filter && matches!(self.view, View::WorkflowList) {
                &self.filter_input
            } else {
                &self.wf_filter_text
            };

        let mut result: Vec<WorkflowSummary> = self
            .workflows
            .iter()
            .filter(|wf| {
                if let Some(active) = self.wf_active_filter {
                    if wf.active != active {
                        return false;
                    }
                }
                if !filter_text.is_empty() {
                    let lower = filter_text.to_lowercase();
                    return wf.name.to_lowercase().contains(&lower)
                        || wf.tag_names().to_lowercase().contains(&lower);
                }
                true
            })
            .cloned()
            .collect();

        // Apply sort
        match &self.wf_sort {
            SortKind::NameAsc => {
                result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }
            SortKind::NameDesc => {
                result.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()))
            }
            SortKind::StatusAsc => result.sort_by_key(|w| !w.active),
            SortKind::StatusDesc => result.sort_by_key(|w| w.active),
            SortKind::UpdatedAsc => result.sort_by_key(|w| w.updated_at),
            SortKind::UpdatedDesc => result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            _ => {}
        }

        self.wf_scroll.set_len(result.len());
        self.cached_filtered_wf = Some(result.clone());
        self.wf_cache_dirty = false;
        result
    }

    pub fn filtered_executions(&mut self) -> Vec<ExecutionSummary> {
        if !self.exec_cache_dirty {
            if let Some(ref cached) = self.cached_filtered_exec {
                return cached.clone();
            }
        }

        let filter_text =
            if self.input_mode == InputMode::Filter && matches!(self.view, View::ExecutionList) {
                &self.filter_input
            } else {
                &self.exec_filter_text
            };

        let mut result: Vec<ExecutionSummary> = self
            .executions
            .iter()
            .filter(|e| {
                if let Some(ref status) = self.exec_status_filter {
                    if e.status.filter_key() != status.as_str() {
                        return false;
                    }
                }
                if !filter_text.is_empty() {
                    let lower = filter_text.to_lowercase();
                    let name_match = e
                        .workflow_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&lower))
                        .unwrap_or(false);
                    let id_match = e.id.contains(&lower);
                    return name_match || id_match;
                }
                true
            })
            .cloned()
            .collect();

        // Apply sort
        match &self.exec_sort {
            SortKind::NameAsc => result.sort_by(|a, b| a.workflow_name.cmp(&b.workflow_name)),
            SortKind::NameDesc => result.sort_by(|a, b| b.workflow_name.cmp(&a.workflow_name)),
            SortKind::StatusAsc => {
                result.sort_by(|a, b| a.status.filter_key().cmp(b.status.filter_key()))
            }
            SortKind::StatusDesc => {
                result.sort_by(|a, b| b.status.filter_key().cmp(a.status.filter_key()))
            }
            SortKind::DurationAsc => result.sort_by_key(|e| {
                e.stopped_at
                    .zip(e.started_at)
                    .map(|(stop, start)| (stop - start).num_milliseconds())
                    .unwrap_or(i64::MAX)
            }),
            SortKind::DurationDesc => result.sort_by(|a, b| {
                let da = a
                    .stopped_at
                    .zip(a.started_at)
                    .map(|(stop, start)| (stop - start).num_milliseconds())
                    .unwrap_or(0);
                let db = b
                    .stopped_at
                    .zip(b.started_at)
                    .map(|(stop, start)| (stop - start).num_milliseconds())
                    .unwrap_or(0);
                db.cmp(&da)
            }),
            _ => {}
        }

        self.exec_scroll.set_len(result.len());
        self.cached_filtered_exec = Some(result.clone());
        self.exec_cache_dirty = false;
        result
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /// Get mutable reference to the active view's scroll state.
    fn active_scroll_mut(&mut self) -> &mut ScrollState {
        match self.view {
            View::WorkflowList => &mut self.wf_scroll,
            View::ExecutionList => &mut self.exec_scroll,
            View::ExecutionDetail => &mut self.node_scroll,
            View::Insights => &mut self.insight_scroll,
            _ => &mut self.wf_scroll, // fallback
        }
    }

    /// Generate a full, working URL for the currently selected item.
    fn current_item_url(&mut self) -> Option<String> {
        let base = self.base_url.trim_end_matches('/').to_string();
        match self.view.clone() {
            View::WorkflowList => {
                let filtered = self.filtered_workflows();
                let sel = self.wf_scroll.selected();
                filtered
                    .get(sel)
                    .map(|wf| format!("{}/workflow/{}", base, wf.id))
            }
            View::WorkflowDetail => self
                .workflow_detail
                .as_ref()
                .map(|wf| format!("{}/workflow/{}", base, wf.id)),
            View::ExecutionList => {
                let filtered = self.filtered_executions();
                let sel = self.exec_scroll.selected();
                filtered
                    .get(sel)
                    .map(|e| format!("{}/executions/{}", base, e.id))
            }
            View::ExecutionDetail | View::NodeDetail(_) => self
                .execution_detail
                .as_ref()
                .map(|e| format!("{}/executions/{}", base, e.id)),
            _ => None,
        }
    }

    /// Sort indicator suffix for column headers.
    pub fn sort_indicator(&self, kind: &SortKind, asc: &SortKind, desc: &SortKind) -> &'static str {
        if kind == asc {
            " ▲"
        } else if kind == desc {
            " ▼"
        } else {
            ""
        }
    }
}
