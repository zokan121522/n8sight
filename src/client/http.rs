use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr};
use reqwest::{Client, Response};

use crate::config::Config;
use crate::domain::execution::{ExecutionDetail, ExecutionListResponse};
use crate::domain::workflow::{WorkflowDetail, WorkflowListResponse};

use super::{ExecutionFilter, N8nClient, WorkflowFilter};

/// Production HTTP client for the n8n REST API.
pub struct HttpN8nClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl HttpN8nClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .wrap_err("Failed to build HTTP client")?;

        Ok(Self {
            client,
            base_url: config.base_url(),
            api_key: config.api_key.clone(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn authed(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .header("X-N8N-API-KEY", &self.api_key)
            .header("Accept", "application/json")
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.authed(self.client.get(self.url(path)))
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.authed(self.client.post(self.url(path)))
    }

    async fn check(resp: Response) -> Result<Response> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp)
        } else {
            let url = resp.url().to_string();
            let body = resp.text().await.unwrap_or_default();
            color_eyre::eyre::bail!(
                "n8n API error {} on {}: {}",
                status.as_u16(),
                url,
                body.chars().take(500).collect::<String>()
            );
        }
    }
}

#[async_trait]
impl N8nClient for HttpN8nClient {
    async fn list_workflows(&self, filter: WorkflowFilter) -> Result<WorkflowListResponse> {
        let mut req = self.get("/workflows");
        if let Some(active) = filter.active {
            req = req.query(&[("active", active.to_string())]);
        }
        if let Some(ref tags) = filter.tags {
            req = req.query(&[("tags", tags.as_str())]);
        }
        if let Some(ref name) = filter.name {
            req = req.query(&[("name", name.as_str())]);
        }
        if let Some(ref project_id) = filter.project_id {
            req = req.query(&[("projectId", project_id.as_str())]);
        }
        if let Some(limit) = filter.limit {
            req = req.query(&[("limit", limit.to_string())]);
        }
        if let Some(ref cursor) = filter.cursor {
            req = req.query(&[("cursor", cursor.as_str())]);
        }
        let resp = req.send().await.wrap_err("Failed to connect to n8n")?;
        Self::check(resp)
            .await?
            .json::<WorkflowListResponse>()
            .await
            .wrap_err("Failed to parse workflow list response")
    }

    async fn get_workflow(&self, id: &str) -> Result<WorkflowDetail> {
        let resp = self
            .get(&format!("/workflows/{}", id))
            .send()
            .await
            .wrap_err("Failed to connect to n8n")?;
        Self::check(resp)
            .await?
            .json::<WorkflowDetail>()
            .await
            .wrap_err("Failed to parse workflow detail")
    }

    async fn activate_workflow(&self, id: &str) -> Result<WorkflowDetail> {
        let resp = self
            .post(&format!("/workflows/{}/activate", id))
            .send()
            .await
            .wrap_err("Failed to connect to n8n")?;
        Self::check(resp)
            .await?
            .json::<WorkflowDetail>()
            .await
            .wrap_err("Failed to parse activate response")
    }

    async fn deactivate_workflow(&self, id: &str) -> Result<WorkflowDetail> {
        let resp = self
            .post(&format!("/workflows/{}/deactivate", id))
            .send()
            .await
            .wrap_err("Failed to connect to n8n")?;
        Self::check(resp)
            .await?
            .json::<WorkflowDetail>()
            .await
            .wrap_err("Failed to parse deactivate response")
    }

    async fn list_executions(&self, filter: ExecutionFilter) -> Result<ExecutionListResponse> {
        let mut req = self.get("/executions");
        if let Some(ref status) = filter.status {
            req = req.query(&[("status", status.as_str())]);
        }
        if let Some(ref wf_id) = filter.workflow_id {
            req = req.query(&[("workflowId", wf_id.as_str())]);
        }
        if let Some(ref project_id) = filter.project_id {
            req = req.query(&[("projectId", project_id.as_str())]);
        }
        if let Some(limit) = filter.limit {
            req = req.query(&[("limit", limit.to_string())]);
        }
        if let Some(ref cursor) = filter.cursor {
            req = req.query(&[("cursor", cursor.as_str())]);
        }
        if filter.include_data {
            req = req.query(&[("includeData", "true")]);
        }
        let resp = req.send().await.wrap_err("Failed to connect to n8n")?;
        Self::check(resp)
            .await?
            .json::<ExecutionListResponse>()
            .await
            .wrap_err("Failed to parse execution list response")
    }

    async fn get_execution(&self, id: &str, include_data: bool) -> Result<ExecutionDetail> {
        let mut req = self.get(&format!("/executions/{}", id));
        if include_data {
            req = req.query(&[("includeData", "true")]);
        }
        let resp = req.send().await.wrap_err("Failed to connect to n8n")?;
        Self::check(resp)
            .await?
            .json::<ExecutionDetail>()
            .await
            .wrap_err("Failed to parse execution detail")
    }

    async fn retry_execution(&self, id: &str, load_workflow: bool) -> Result<ExecutionDetail> {
        let body = serde_json::json!({ "loadWorkflow": load_workflow });
        let resp = self
            .post(&format!("/executions/{}/retry", id))
            .json(&body)
            .send()
            .await
            .wrap_err("Failed to connect to n8n")?;
        Self::check(resp)
            .await?
            .json::<ExecutionDetail>()
            .await
            .wrap_err("Failed to parse retry response")
    }
}
