use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt;

/// Severity level for an insight finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Critical => write!(f, "CRIT"),
        }
    }
}

impl Severity {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Info => "ℹ",
            Self::Warning => "⚠",
            Self::Critical => "✗",
        }
    }
}

/// Category of an insight finding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum InsightCategory {
    HighFailureRate,
    StuckExecution,
    RetryStorm,
    #[allow(dead_code)]
    NodeFailureHotspot,
    LongRunningExecution,
    InactiveCriticalWorkflow,
    AbandonedWorkflow,
    #[allow(dead_code)]
    CredentialIssue,
}

impl fmt::Display for InsightCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HighFailureRate => write!(f, "High Failure Rate"),
            Self::StuckExecution => write!(f, "Stuck Execution"),
            Self::RetryStorm => write!(f, "Retry Storm"),
            Self::NodeFailureHotspot => write!(f, "Node Failure Hotspot"),
            Self::LongRunningExecution => write!(f, "Long Running Execution"),
            Self::InactiveCriticalWorkflow => write!(f, "Inactive Critical Workflow"),
            Self::AbandonedWorkflow => write!(f, "Abandoned Workflow"),
            Self::CredentialIssue => write!(f, "Credential Issue"),
        }
    }
}

/// A single insight finding.
#[derive(Debug, Clone, Serialize)]
pub struct InsightFinding {
    pub severity: Severity,
    pub category: InsightCategory,
    pub title: String,
    pub detail: String,
    pub affected_entity: String,
    pub computed_at: DateTime<Utc>,
}

/// Result of an insights scan.
#[derive(Debug, Clone, Serialize)]
pub struct InsightsResult {
    pub findings: Vec<InsightFinding>,
    pub workflows_scanned: usize,
    pub executions_scanned: usize,
    pub scan_duration_ms: u64,
    pub computed_at: DateTime<Utc>,
}

impl InsightsResult {
    pub fn critical_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count()
    }
}
