use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Execution status values from the n8n API.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Success,
    Error,
    Running,
    Waiting,
    Canceled,
    #[serde(other)]
    Unknown,
}

impl fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "Success"),
            Self::Error => write!(f, "Error"),
            Self::Running => write!(f, "Running"),
            Self::Waiting => write!(f, "Waiting"),
            Self::Canceled => write!(f, "Canceled"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl ExecutionStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Success => "✓",
            Self::Error => "✗",
            Self::Running => "⟳",
            Self::Waiting => "◷",
            Self::Canceled => "⊘",
            Self::Unknown => "?",
        }
    }

    pub fn filter_key(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Canceled => "canceled",
            Self::Unknown => "",
        }
    }
}

/// Summary of an execution as returned by the list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub id: String,
    #[serde(default)]
    pub finished: bool,
    #[serde(default)]
    pub mode: String,
    pub status: ExecutionStatus,
    #[serde(rename = "startedAt")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(rename = "stoppedAt")]
    pub stopped_at: Option<DateTime<Utc>>,
    #[serde(rename = "workflowId")]
    pub workflow_id: Option<String>,
    #[serde(rename = "workflowName", default)]
    pub workflow_name: Option<String>,
    #[serde(rename = "retryOf")]
    pub retry_of: Option<String>,
    #[serde(rename = "retrySuccessId")]
    pub retry_success_id: Option<String>,
}

/// Full execution detail including per-node data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDetail {
    pub id: String,
    #[serde(default)]
    pub finished: bool,
    #[serde(default)]
    pub mode: String,
    pub status: ExecutionStatus,
    #[serde(rename = "startedAt")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(rename = "stoppedAt")]
    pub stopped_at: Option<DateTime<Utc>>,
    #[serde(rename = "workflowId")]
    pub workflow_id: Option<String>,
    #[serde(rename = "retryOf")]
    pub retry_of: Option<String>,
    #[serde(rename = "retrySuccessId")]
    pub retry_success_id: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(rename = "workflowData")]
    pub workflow_data: Option<serde_json::Value>,
}

/// Parsed per-node execution result.
#[derive(Debug, Clone, Serialize)]
pub struct NodeRunResult {
    pub node_name: String,
    pub status: NodeRunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub items_in: usize,
    pub items_out: usize,
    pub error: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum NodeRunStatus {
    Success,
    Error,
    Waiting,
    #[allow(dead_code)]
    Unknown,
}

impl fmt::Display for NodeRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "✓"),
            Self::Error => write!(f, "✗"),
            Self::Waiting => write!(f, "◷"),
            Self::Unknown => write!(f, "?"),
        }
    }
}

/// n8n paginated response wrapper for executions.
#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionListResponse {
    pub data: Vec<ExecutionSummary>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

impl ExecutionSummary {
    /// Duration in a human-readable format.
    /// Takes `now` as a parameter to avoid calling Utc::now() in render paths.
    pub fn duration_display(&self, now: DateTime<Utc>) -> String {
        match (self.started_at, self.stopped_at) {
            (Some(start), Some(stop)) => {
                let dur = stop - start;
                format_duration(dur.num_milliseconds())
            }
            (Some(start), None) if self.status == ExecutionStatus::Running => {
                let dur = now - start;
                format!("{}…", format_duration(dur.num_milliseconds()))
            }
            _ => "—".to_string(),
        }
    }

    /// Relative time since started. Takes `now` as parameter.
    pub fn started_ago(&self, now: DateTime<Utc>) -> String {
        match self.started_at {
            Some(t) => format_relative(t, now),
            None => "—".to_string(),
        }
    }
}

impl ExecutionDetail {
    /// Extract per-node run results from the execution data JSON.
    /// This is intended to be called once and cached — not called on every frame.
    pub fn node_runs(&self) -> Vec<NodeRunResult> {
        let mut results = Vec::new();

        let run_data = self
            .data
            .as_ref()
            .and_then(|d| d.get("resultData"))
            .and_then(|rd| rd.get("runData"));

        let run_data = match run_data {
            Some(serde_json::Value::Object(map)) => map,
            _ => return results,
        };

        for (node_name, runs) in run_data {
            let runs = match runs.as_array() {
                Some(a) => a,
                None => continue,
            };

            for run in runs {
                let started_at = run
                    .get("startTime")
                    .and_then(|v| v.as_i64())
                    .and_then(DateTime::from_timestamp_millis);

                let duration_ms = run.get("executionTime").and_then(|v| v.as_i64());

                let error = run.get("error").and_then(|e| {
                    e.get("message")
                        .and_then(|m| m.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| serde_json::to_string(e).ok())
                });

                let status = if error.is_some() {
                    NodeRunStatus::Error
                } else {
                    match run.get("executionStatus").and_then(|s| s.as_str()) {
                        Some("success") => NodeRunStatus::Success,
                        Some("error") => NodeRunStatus::Error,
                        Some("waiting") => NodeRunStatus::Waiting,
                        _ => NodeRunStatus::Success,
                    }
                };

                let (items_in, items_out) = parse_items(run);

                results.push(NodeRunResult {
                    node_name: node_name.clone(),
                    status,
                    started_at,
                    duration_ms,
                    items_in,
                    items_out,
                    error,
                    data: Some(run.clone()),
                });
            }
        }

        results.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        results
    }

    /// Duration. Takes `now` for running executions.
    pub fn duration_display(&self, now: DateTime<Utc>) -> String {
        match (self.started_at, self.stopped_at) {
            (Some(start), Some(stop)) => format_duration((stop - start).num_milliseconds()),
            (Some(start), None) => {
                let dur = now - start;
                format!("{}…", format_duration(dur.num_milliseconds()))
            }
            _ => "—".to_string(),
        }
    }
}

fn parse_items(run: &serde_json::Value) -> (usize, usize) {
    let items_in = run
        .get("inputDataItems")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let items_out = run
        .get("data")
        .and_then(|d| d.get("main"))
        .and_then(|m| m.as_array())
        .map(|branches| {
            branches
                .iter()
                .filter_map(|b| b.as_array())
                .map(|a| a.len())
                .sum()
        })
        .unwrap_or(0);

    (items_in, items_out)
}

pub fn format_duration(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        format!("{}m {}s", ms / 60_000, (ms % 60_000) / 1000)
    } else {
        format!("{}h {}m", ms / 3_600_000, (ms % 3_600_000) / 60_000)
    }
}

/// Format a time relative to `now`. Does not call Utc::now() internally.
pub fn format_relative(t: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = (now - t).num_seconds();
    if secs < 0 {
        "in the future".to_string()
    } else if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

/// Status counts across executions.
#[derive(Debug, Clone, Default, Serialize)]
pub struct StatusCounts {
    pub success: u32,
    pub error: u32,
    pub running: u32,
    pub waiting: u32,
    pub canceled: u32,
    pub total: u32,
}

impl StatusCounts {
    pub fn from_executions(execs: &[ExecutionSummary]) -> Self {
        let mut counts = Self::default();
        for e in execs {
            match e.status {
                ExecutionStatus::Success => counts.success += 1,
                ExecutionStatus::Error => counts.error += 1,
                ExecutionStatus::Running => counts.running += 1,
                ExecutionStatus::Waiting => counts.waiting += 1,
                ExecutionStatus::Canceled => counts.canceled += 1,
                ExecutionStatus::Unknown => {}
            }
            counts.total += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(50), "50ms");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(65_000), "1m 5s");
        assert_eq!(format_duration(3_665_000), "1h 1m");
    }

    #[test]
    fn test_format_relative() {
        let now = Utc::now();
        assert_eq!(
            format_relative(now - chrono::Duration::seconds(30), now),
            "just now"
        );
        assert_eq!(
            format_relative(now - chrono::Duration::minutes(5), now),
            "5m ago"
        );
        assert_eq!(
            format_relative(now - chrono::Duration::hours(3), now),
            "3h ago"
        );
        assert_eq!(
            format_relative(now - chrono::Duration::days(2), now),
            "2d ago"
        );
    }

    #[test]
    fn test_status_counts() {
        let execs = vec![
            ExecutionSummary {
                id: "1".into(),
                finished: true,
                mode: "production".into(),
                status: ExecutionStatus::Success,
                started_at: None,
                stopped_at: None,
                workflow_id: None,
                workflow_name: None,
                retry_of: None,
                retry_success_id: None,
            },
            ExecutionSummary {
                id: "2".into(),
                finished: true,
                mode: "production".into(),
                status: ExecutionStatus::Error,
                started_at: None,
                stopped_at: None,
                workflow_id: None,
                workflow_name: None,
                retry_of: None,
                retry_success_id: None,
            },
            ExecutionSummary {
                id: "3".into(),
                finished: true,
                mode: "production".into(),
                status: ExecutionStatus::Success,
                started_at: None,
                stopped_at: None,
                workflow_id: None,
                workflow_name: None,
                retry_of: None,
                retry_success_id: None,
            },
        ];
        let counts = StatusCounts::from_executions(&execs);
        assert_eq!(counts.success, 2);
        assert_eq!(counts.error, 1);
        assert_eq!(counts.total, 3);
    }

    #[test]
    fn test_filter_key_roundtrip() {
        assert_eq!(ExecutionStatus::Success.filter_key(), "success");
        assert_eq!(ExecutionStatus::Error.filter_key(), "error");
        assert_eq!(ExecutionStatus::Running.filter_key(), "running");
    }
}
