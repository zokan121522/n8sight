pub mod http;
pub mod mock;

use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::domain::execution::{ExecutionDetail, ExecutionListResponse, ExecutionSummary};
use crate::domain::workflow::{WorkflowDetail, WorkflowListResponse};

/// Filter parameters for listing executions.
#[derive(Debug, Clone, Default)]
pub struct ExecutionFilter {
    pub status: Option<String>,
    pub workflow_id: Option<String>,
    pub project_id: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub include_data: bool,
}

/// Filter parameters for listing workflows.
#[derive(Debug, Clone, Default)]
pub struct WorkflowFilter {
    pub active: Option<bool>,
    pub tags: Option<String>,
    pub name: Option<String>,
    pub project_id: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

/// Abstraction over the n8n API.
#[async_trait]
pub trait N8nClient: Send + Sync {
    // -- Workflows --
    async fn list_workflows(&self, filter: WorkflowFilter) -> Result<WorkflowListResponse>;
    async fn get_workflow(&self, id: &str) -> Result<WorkflowDetail>;
    async fn activate_workflow(&self, id: &str) -> Result<WorkflowDetail>;
    async fn deactivate_workflow(&self, id: &str) -> Result<WorkflowDetail>;

    // -- Executions --
    async fn list_executions(&self, filter: ExecutionFilter) -> Result<ExecutionListResponse>;
    async fn get_execution(&self, id: &str, include_data: bool) -> Result<ExecutionDetail>;
    async fn retry_execution(&self, id: &str, load_workflow: bool) -> Result<ExecutionDetail>;

    // -- Webhook trigger --
    /// Trigger a webhook with a JSON payload. Returns the execution ID if successful.
    async fn trigger_webhook(&self, webhook_path: &str, json_body: &str) -> Result<String>;

    // -- Utility --
    async fn list_all_executions(
        &self,
        filter: ExecutionFilter,
        max_pages: usize,
    ) -> Result<Vec<ExecutionSummary>> {
        let mut all = Vec::new();
        let mut cursor = filter.cursor.clone();
        let limit = filter.limit.unwrap_or(100);

        for _ in 0..max_pages {
            let mut f = filter.clone();
            f.cursor = cursor;
            f.limit = Some(limit);

            let resp = self.list_executions(f).await?;
            all.extend(resp.data);

            match resp.next_cursor {
                Some(c) if !c.is_empty() => cursor = Some(c),
                _ => break,
            }
        }

        Ok(all)
    }

    /// Fetch all workflows across pages, up to max_pages.
    async fn list_all_workflows(
        &self,
        filter: WorkflowFilter,
        max_pages: usize,
    ) -> Result<Vec<crate::domain::workflow::WorkflowSummary>> {
        let mut all = Vec::new();
        let mut cursor = filter.cursor.clone();
        let limit = filter.limit.unwrap_or(100);

        for _ in 0..max_pages {
            let mut f = filter.clone();
            f.cursor = cursor;
            f.limit = Some(limit);

            let resp = self.list_workflows(f).await?;
            all.extend(resp.data);

            match resp.next_cursor {
                Some(c) if !c.is_empty() => cursor = Some(c),
                _ => break,
            }
        }

        Ok(all)
    }
}
