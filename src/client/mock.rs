use async_trait::async_trait;
use chrono::Utc;
use color_eyre::eyre::Result;

use crate::domain::execution::*;
use crate::domain::workflow::*;

use super::{ExecutionFilter, N8nClient, WorkflowFilter};

/// Mock client for development and testing without a live n8n instance.
pub struct MockN8nClient;

impl MockN8nClient {
    pub fn new() -> Self {
        Self
    }

    fn mock_workflows() -> Vec<WorkflowSummary> {
        let now = Utc::now();
        vec![
            WorkflowSummary {
                id: "1".to_string(),
                name: "Customer Onboarding".to_string(),
                active: true,
                tags: vec![Tag {
                    id: "1".to_string(),
                    name: "production".to_string(),
                    created_at: Some(now),
                    updated_at: Some(now),
                }],
                created_at: Some(now - chrono::Duration::days(30)),
                updated_at: Some(now - chrono::Duration::hours(2)),
                version_id: Some("v1".to_string()),
            },
            WorkflowSummary {
                id: "2".to_string(),
                name: "Slack Notification Bot".to_string(),
                active: true,
                tags: vec![Tag {
                    id: "2".to_string(),
                    name: "integrations".to_string(),
                    created_at: Some(now),
                    updated_at: Some(now),
                }],
                created_at: Some(now - chrono::Duration::days(14)),
                updated_at: Some(now - chrono::Duration::hours(6)),
                version_id: Some("v3".to_string()),
            },
            WorkflowSummary {
                id: "3".to_string(),
                name: "Data Sync Pipeline".to_string(),
                active: false,
                tags: vec![],
                created_at: Some(now - chrono::Duration::days(60)),
                updated_at: Some(now - chrono::Duration::days(45)),
                version_id: Some("v2".to_string()),
            },
            WorkflowSummary {
                id: "4".to_string(),
                name: "Invoice Generator".to_string(),
                active: true,
                tags: vec![Tag {
                    id: "1".to_string(),
                    name: "production".to_string(),
                    created_at: Some(now),
                    updated_at: Some(now),
                }],
                created_at: Some(now - chrono::Duration::days(7)),
                updated_at: Some(now - chrono::Duration::minutes(30)),
                version_id: Some("v5".to_string()),
            },
            WorkflowSummary {
                id: "5".to_string(),
                name: "Weekly Report Emailer".to_string(),
                active: true,
                tags: vec![Tag {
                    id: "3".to_string(),
                    name: "scheduled".to_string(),
                    created_at: Some(now),
                    updated_at: Some(now),
                }],
                created_at: Some(now - chrono::Duration::days(90)),
                updated_at: Some(now - chrono::Duration::days(1)),
                version_id: Some("v1".to_string()),
            },
        ]
    }

    fn mock_executions() -> Vec<ExecutionSummary> {
        let now = Utc::now();
        vec![
            ExecutionSummary {
                id: "1001".to_string(),
                finished: true,
                mode: "production".to_string(),
                status: ExecutionStatus::Success,
                started_at: Some(now - chrono::Duration::minutes(5)),
                stopped_at: Some(now - chrono::Duration::minutes(4)),
                workflow_id: Some("1".to_string()),
                workflow_name: Some("Customer Onboarding".to_string()),
                retry_of: None,
                retry_success_id: None,
            },
            ExecutionSummary {
                id: "1002".to_string(),
                finished: true,
                mode: "production".to_string(),
                status: ExecutionStatus::Error,
                started_at: Some(now - chrono::Duration::minutes(10)),
                stopped_at: Some(now - chrono::Duration::minutes(9)),
                workflow_id: Some("2".to_string()),
                workflow_name: Some("Slack Notification Bot".to_string()),
                retry_of: None,
                retry_success_id: None,
            },
            ExecutionSummary {
                id: "1003".to_string(),
                finished: false,
                mode: "production".to_string(),
                status: ExecutionStatus::Running,
                started_at: Some(now - chrono::Duration::seconds(30)),
                stopped_at: None,
                workflow_id: Some("4".to_string()),
                workflow_name: Some("Invoice Generator".to_string()),
                retry_of: None,
                retry_success_id: None,
            },
            ExecutionSummary {
                id: "1004".to_string(),
                finished: true,
                mode: "manual".to_string(),
                status: ExecutionStatus::Success,
                started_at: Some(now - chrono::Duration::hours(1)),
                stopped_at: Some(now - chrono::Duration::minutes(59)),
                workflow_id: Some("1".to_string()),
                workflow_name: Some("Customer Onboarding".to_string()),
                retry_of: None,
                retry_success_id: None,
            },
            ExecutionSummary {
                id: "1005".to_string(),
                finished: false,
                mode: "production".to_string(),
                status: ExecutionStatus::Waiting,
                started_at: Some(now - chrono::Duration::hours(2)),
                stopped_at: None,
                workflow_id: Some("5".to_string()),
                workflow_name: Some("Weekly Report Emailer".to_string()),
                retry_of: None,
                retry_success_id: None,
            },
            ExecutionSummary {
                id: "1006".to_string(),
                finished: true,
                mode: "production".to_string(),
                status: ExecutionStatus::Error,
                started_at: Some(now - chrono::Duration::hours(3)),
                stopped_at: Some(now - chrono::Duration::hours(2) - chrono::Duration::minutes(58)),
                workflow_id: Some("2".to_string()),
                workflow_name: Some("Slack Notification Bot".to_string()),
                retry_of: None,
                retry_success_id: Some("1007".to_string()),
            },
            ExecutionSummary {
                id: "1007".to_string(),
                finished: true,
                mode: "production".to_string(),
                status: ExecutionStatus::Success,
                started_at: Some(now - chrono::Duration::hours(2) - chrono::Duration::minutes(55)),
                stopped_at: Some(now - chrono::Duration::hours(2) - chrono::Duration::minutes(54)),
                workflow_id: Some("2".to_string()),
                workflow_name: Some("Slack Notification Bot".to_string()),
                retry_of: Some("1006".to_string()),
                retry_success_id: None,
            },
        ]
    }
}

#[async_trait]
impl N8nClient for MockN8nClient {
    async fn list_workflows(&self, filter: WorkflowFilter) -> Result<WorkflowListResponse> {
        let mut workflows = Self::mock_workflows();

        if let Some(active) = filter.active {
            workflows.retain(|w| w.active == active);
        }
        if let Some(ref name) = filter.name {
            let name_lower = name.to_lowercase();
            workflows.retain(|w| w.name.to_lowercase().contains(&name_lower));
        }

        Ok(WorkflowListResponse {
            data: workflows,
            next_cursor: None,
        })
    }

    async fn get_workflow(&self, id: &str) -> Result<WorkflowDetail> {
        let summary = Self::mock_workflows()
            .into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| color_eyre::eyre::eyre!("Workflow not found: {}", id))?;

        Ok(WorkflowDetail {
            id: summary.id,
            name: summary.name,
            active: summary.active,
            tags: summary.tags,
            created_at: summary.created_at,
            updated_at: summary.updated_at,
            version_id: summary.version_id,
            nodes: vec![
                WorkflowNode {
                    name: "Webhook".to_string(),
                    node_type: "n8n-nodes-base.webhook".to_string(),
                    position: vec![250.0, 300.0],
                    parameters: serde_json::json!({"path": "/onboard"}),
                    credentials: None,
                    disabled: false,
                    type_version: Some(serde_json::json!(1)),
                },
                WorkflowNode {
                    name: "Set Fields".to_string(),
                    node_type: "n8n-nodes-base.set".to_string(),
                    position: vec![450.0, 300.0],
                    parameters: serde_json::json!({}),
                    credentials: None,
                    disabled: false,
                    type_version: Some(serde_json::json!(1)),
                },
                WorkflowNode {
                    name: "IF".to_string(),
                    node_type: "n8n-nodes-base.if".to_string(),
                    position: vec![650.0, 300.0],
                    parameters: serde_json::json!({}),
                    credentials: None,
                    disabled: false,
                    type_version: Some(serde_json::json!(1)),
                },
                WorkflowNode {
                    name: "Send Email".to_string(),
                    node_type: "n8n-nodes-base.emailSend".to_string(),
                    position: vec![850.0, 200.0],
                    parameters: serde_json::json!({}),
                    credentials: Some(serde_json::json!({"smtp": {"id": "1", "name": "SMTP"}})),
                    disabled: false,
                    type_version: Some(serde_json::json!(1)),
                },
                WorkflowNode {
                    name: "Slack".to_string(),
                    node_type: "n8n-nodes-base.slack".to_string(),
                    position: vec![850.0, 400.0],
                    parameters: serde_json::json!({"channel": "#onboarding"}),
                    credentials: Some(
                        serde_json::json!({"slackApi": {"id": "2", "name": "Slack"}}),
                    ),
                    disabled: false,
                    type_version: Some(serde_json::json!(1)),
                },
            ],
            connections: serde_json::json!({
                "Webhook": {"main": [[{"node": "Set Fields", "type": "main", "index": 0}]]},
                "Set Fields": {"main": [[{"node": "IF", "type": "main", "index": 0}]]},
                "IF": {"main": [
                    [{"node": "Send Email", "type": "main", "index": 0}],
                    [{"node": "Slack", "type": "main", "index": 0}]
                ]}
            }),
            settings: serde_json::json!({"executionOrder": "v1"}),
            static_data: None,
        })
    }

    async fn activate_workflow(&self, id: &str) -> Result<WorkflowDetail> {
        self.get_workflow(id).await
    }

    async fn deactivate_workflow(&self, id: &str) -> Result<WorkflowDetail> {
        self.get_workflow(id).await
    }

    async fn list_executions(&self, filter: ExecutionFilter) -> Result<ExecutionListResponse> {
        let mut executions = Self::mock_executions();

        if let Some(ref status) = filter.status {
            executions.retain(|e| e.status.filter_key() == status.as_str());
        }
        if let Some(ref wf_id) = filter.workflow_id {
            executions.retain(|e| e.workflow_id.as_deref() == Some(wf_id.as_str()));
        }

        Ok(ExecutionListResponse {
            data: executions,
            next_cursor: None,
        })
    }

    async fn get_execution(&self, id: &str, _include_data: bool) -> Result<ExecutionDetail> {
        let summary = Self::mock_executions()
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(|| color_eyre::eyre::eyre!("Execution not found: {}", id))?;

        let mock_data = serde_json::json!({
            "resultData": {
                "runData": {
                    "Webhook": [{
                        "startTime": chrono::Utc::now().timestamp_millis() - 5000,
                        "executionTime": 12,
                        "executionStatus": "success",
                        "data": {"main": [[{"json": {"name": "Test User"}}]]}
                    }],
                    "Set Fields": [{
                        "startTime": chrono::Utc::now().timestamp_millis() - 4900,
                        "executionTime": 3,
                        "executionStatus": "success",
                        "data": {"main": [[{"json": {"name": "Test User", "processed": true}}]]}
                    }],
                    "IF": [{
                        "startTime": chrono::Utc::now().timestamp_millis() - 4800,
                        "executionTime": 1,
                        "executionStatus": "success",
                        "data": {"main": [[{"json": {"name": "Test User"}}], []]}
                    }],
                    "Send Email": [{
                        "startTime": chrono::Utc::now().timestamp_millis() - 4700,
                        "executionTime": 450,
                        "executionStatus": "success",
                        "data": {"main": [[{"json": {"messageId": "abc123"}}]]}
                    }]
                }
            }
        });

        Ok(ExecutionDetail {
            id: summary.id,
            finished: summary.finished,
            mode: summary.mode,
            status: summary.status,
            started_at: summary.started_at,
            stopped_at: summary.stopped_at,
            workflow_id: summary.workflow_id,
            retry_of: summary.retry_of,
            retry_success_id: summary.retry_success_id,
            data: Some(mock_data),
            workflow_data: None,
        })
    }

    async fn trigger_webhook(&self, _webhook_path: &str, json_body: &str) -> Result<String> {
        // Simulate triggering — always "succeeds" and returns a fake execution ID
        let _body: serde_json::Value = serde_json::from_str(json_body)
            .map_err(|e| color_eyre::eyre::eyre!("Mock: invalid JSON: {}", e))?;
        Ok("mock-exec-2001".to_string())
    }

    async fn retry_execution(&self, id: &str, _load_workflow: bool) -> Result<ExecutionDetail> {
        self.get_execution(id, false).await
    }

}
