use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Summary of a workflow as returned by the list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub active: bool,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "versionId")]
    pub version_id: Option<String>,
}

/// Full workflow detail including nodes and connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDetail {
    pub id: String,
    pub name: String,
    pub active: bool,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "versionId")]
    pub version_id: Option<String>,
    #[serde(default)]
    pub nodes: Vec<WorkflowNode>,
    #[serde(default)]
    pub connections: serde_json::Value,
    #[serde(default)]
    pub settings: serde_json::Value,
    #[serde(rename = "staticData", default)]
    pub static_data: Option<serde_json::Value>,
}

/// A node within a workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    /// Node display name
    pub name: String,
    /// Node type (e.g., "n8n-nodes-base.webhook")
    #[serde(rename = "type")]
    pub node_type: String,
    /// Position on canvas [x, y]
    #[serde(default)]
    pub position: Vec<f64>,
    /// Node parameters/configuration
    #[serde(default)]
    pub parameters: serde_json::Value,
    /// Credential references
    #[serde(default)]
    pub credentials: Option<serde_json::Value>,
    /// Whether the node is disabled
    #[serde(default)]
    pub disabled: bool,
    /// Type version
    #[serde(rename = "typeVersion", default)]
    pub type_version: Option<serde_json::Value>,
}

/// Tag attached to a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// n8n paginated response wrapper for workflows.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowListResponse {
    pub data: Vec<WorkflowSummary>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

impl WorkflowSummary {
    pub fn tag_names(&self) -> String {
        self.tags
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl WorkflowDetail {
    /// Short type name for a node (strip "n8n-nodes-base." prefix)
    pub fn short_node_type(node_type: &str) -> String {
        node_type
            .strip_prefix("n8n-nodes-base.")
            .or_else(|| node_type.strip_prefix("@n8n/n8n-nodes-"))
            .unwrap_or(node_type)
            .to_string()
    }

    #[allow(dead_code)]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn trigger_nodes(&self) -> Vec<&WorkflowNode> {
        self.nodes
            .iter()
            .filter(|n| {
                n.node_type.contains("trigger")
                    || n.node_type.contains("webhook")
                    || n.node_type.contains("cron")
                    || n.node_type.contains("schedule")
            })
            .collect()
    }
}
