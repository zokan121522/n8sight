use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;

use crate::action::Action;
use crate::client::{ExecutionFilter, N8nClient, WorkflowFilter};
use crate::domain::insights::InsightsResult;
use crate::domain::insights_compute;

/// Request types that the CLI worker can process.
#[derive(Debug)]
pub enum WorkerRequest {
    FetchWorkflows(WorkflowFilter),
    FetchWorkflowDetail(String),
    FetchExecutions(ExecutionFilter),
    FetchExecutionDetail(String, bool),
    RunInsights(usize),
    ActivateWorkflow(String),
    DeactivateWorkflow(String),
    RetryExecution(String),
}

/// The CLI worker serializes async API calls and sends results back as Actions.
pub struct CliWorker {
    client: Arc<dyn N8nClient>,
    request_rx: mpsc::UnboundedReceiver<WorkerRequest>,
    action_tx: mpsc::UnboundedSender<Action>,
}

impl CliWorker {
    pub fn new(
        client: Arc<dyn N8nClient>,
        request_rx: mpsc::UnboundedReceiver<WorkerRequest>,
        action_tx: mpsc::UnboundedSender<Action>,
    ) -> Self {
        Self {
            client,
            request_rx,
            action_tx,
        }
    }

    pub async fn run(mut self) {
        while let Some(request) = self.request_rx.recv().await {
            let action = self.process(request).await;
            if self.action_tx.send(action).is_err() {
                error!("Action channel closed — TUI has stopped");
                break;
            }
        }
    }

    async fn process(&self, request: WorkerRequest) -> Action {
        match request {
            WorkerRequest::FetchWorkflows(filter) => {
                match self.client.list_all_workflows(filter, 5).await {
                    Ok(workflows) => Action::WorkflowsLoaded(workflows),
                    Err(e) => Action::LoadError(format!("Failed to fetch workflows: {}", e)),
                }
            }

            WorkerRequest::FetchWorkflowDetail(id) => match self.client.get_workflow(&id).await {
                Ok(detail) => Action::WorkflowDetailLoaded(Box::new(detail)),
                Err(e) => Action::LoadError(format!("Failed to fetch workflow: {}", e)),
            },

            WorkerRequest::FetchExecutions(filter) => {
                match self.client.list_all_executions(filter, 5).await {
                    Ok(executions) => Action::ExecutionsLoaded(executions),
                    Err(e) => Action::LoadError(format!("Failed to fetch executions: {}", e)),
                }
            }

            WorkerRequest::FetchExecutionDetail(id, include_data) => {
                match self.client.get_execution(&id, include_data).await {
                    Ok(detail) => Action::ExecutionDetailLoaded(Box::new(detail)),
                    Err(e) => Action::LoadError(format!("Failed to fetch execution: {}", e)),
                }
            }

            WorkerRequest::RunInsights(max_pages) => {
                let wf_filter = WorkflowFilter {
                    limit: Some(100),
                    ..Default::default()
                };
                let exec_filter = ExecutionFilter {
                    limit: Some(100),
                    ..Default::default()
                };

                let workflows = match self.client.list_all_workflows(wf_filter, max_pages).await {
                    Ok(w) => w,
                    Err(e) => return Action::LoadError(format!("Insights: {}", e)),
                };

                let executions = match self
                    .client
                    .list_all_executions(exec_filter, max_pages)
                    .await
                {
                    Ok(e) => e,
                    Err(e) => return Action::LoadError(format!("Insights: {}", e)),
                };

                let start = std::time::Instant::now();
                let findings = insights_compute::compute_insights(&workflows, &executions);
                let scan_duration = start.elapsed().as_millis() as u64;

                Action::InsightsLoaded(Box::new(InsightsResult {
                    findings,
                    workflows_scanned: workflows.len(),
                    executions_scanned: executions.len(),
                    scan_duration_ms: scan_duration,
                    computed_at: chrono::Utc::now(),
                }))
            }

            WorkerRequest::ActivateWorkflow(id) => match self.client.activate_workflow(&id).await {
                Ok(_) => {
                    match self
                        .client
                        .list_all_workflows(WorkflowFilter::default(), 5)
                        .await
                    {
                        Ok(workflows) => Action::WorkflowsLoaded(workflows),
                        Err(e) => Action::LoadError(e.to_string()),
                    }
                }
                Err(e) => Action::LoadError(format!("Failed to activate: {}", e)),
            },

            WorkerRequest::DeactivateWorkflow(id) => {
                match self.client.deactivate_workflow(&id).await {
                    Ok(_) => {
                        match self
                            .client
                            .list_all_workflows(WorkflowFilter::default(), 5)
                            .await
                        {
                            Ok(workflows) => Action::WorkflowsLoaded(workflows),
                            Err(e) => Action::LoadError(e.to_string()),
                        }
                    }
                    Err(e) => Action::LoadError(format!("Failed to deactivate: {}", e)),
                }
            }

            WorkerRequest::RetryExecution(id) => {
                match self.client.retry_execution(&id, false).await {
                    Ok(_) => {
                        match self
                            .client
                            .list_all_executions(ExecutionFilter::default(), 5)
                            .await
                        {
                            Ok(executions) => Action::ExecutionsLoaded(executions),
                            Err(e) => Action::LoadError(e.to_string()),
                        }
                    }
                    Err(e) => Action::LoadError(format!("Failed to retry: {}", e)),
                }
            }
        }
    }
}
