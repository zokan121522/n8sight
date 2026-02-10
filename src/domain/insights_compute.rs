use chrono::Utc;
use std::collections::HashMap;

use super::execution::{ExecutionStatus, ExecutionSummary};
use super::insights::*;
use super::workflow::WorkflowSummary;

/// Compute insights from workflows and executions data.
pub fn compute_insights(
    workflows: &[WorkflowSummary],
    executions: &[ExecutionSummary],
) -> Vec<InsightFinding> {
    let now = Utc::now();
    let mut findings = Vec::new();

    findings.extend(detect_high_failure_rate(executions, now));
    findings.extend(detect_stuck_executions(executions, now));
    findings.extend(detect_retry_storms(executions, now));
    findings.extend(detect_long_running(executions, now));
    findings.extend(detect_abandoned_workflows(workflows, executions, now));
    findings.extend(detect_inactive_critical(workflows, now));

    // Sort by severity (Critical first)
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));

    findings
}

/// Detect workflows with a high failure rate.
fn detect_high_failure_rate(
    executions: &[ExecutionSummary],
    now: chrono::DateTime<Utc>,
) -> Vec<InsightFinding> {
    let mut findings = Vec::new();

    // Group executions by workflow
    let mut by_workflow: HashMap<String, Vec<&ExecutionSummary>> = HashMap::new();
    for e in executions {
        if let Some(ref wf_id) = e.workflow_id {
            by_workflow.entry(wf_id.clone()).or_default().push(e);
        }
    }

    for (wf_id, execs) in &by_workflow {
        if execs.len() < 3 {
            continue; // Need enough data
        }

        let error_count = execs
            .iter()
            .filter(|e| e.status == ExecutionStatus::Error)
            .count();
        let error_rate = error_count as f64 / execs.len() as f64;

        let wf_name = execs
            .first()
            .and_then(|e| e.workflow_name.clone())
            .unwrap_or_else(|| wf_id.clone());

        if error_rate > 0.5 {
            findings.push(InsightFinding {
                severity: Severity::Critical,
                category: InsightCategory::HighFailureRate,
                title: format!("High failure rate: {}", wf_name),
                detail: format!(
                    "{}/{} executions failed ({:.0}%) for workflow '{}'",
                    error_count,
                    execs.len(),
                    error_rate * 100.0,
                    wf_name
                ),
                affected_entity: format!("workflow:{}", wf_id),
                computed_at: now,
            });
        } else if error_rate > 0.2 {
            findings.push(InsightFinding {
                severity: Severity::Warning,
                category: InsightCategory::HighFailureRate,
                title: format!("Elevated failure rate: {}", wf_name),
                detail: format!(
                    "{}/{} executions failed ({:.0}%) for workflow '{}'",
                    error_count,
                    execs.len(),
                    error_rate * 100.0,
                    wf_name
                ),
                affected_entity: format!("workflow:{}", wf_id),
                computed_at: now,
            });
        }
    }

    findings
}

/// Detect executions stuck in Running or Waiting state.
fn detect_stuck_executions(
    executions: &[ExecutionSummary],
    now: chrono::DateTime<Utc>,
) -> Vec<InsightFinding> {
    let mut findings = Vec::new();
    let stuck_threshold_minutes = 30;

    for e in executions {
        if e.status != ExecutionStatus::Running && e.status != ExecutionStatus::Waiting {
            continue;
        }

        if let Some(started) = e.started_at {
            let elapsed_mins = (now - started).num_minutes();
            if elapsed_mins > stuck_threshold_minutes {
                let wf_name = e
                    .workflow_name
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());

                let severity = if elapsed_mins > 120 {
                    Severity::Critical
                } else {
                    Severity::Warning
                };

                findings.push(InsightFinding {
                    severity,
                    category: InsightCategory::StuckExecution,
                    title: format!("Execution {} {} for {}m", e.id, e.status, elapsed_mins),
                    detail: format!(
                        "Execution {} of workflow '{}' has been in {} state for {} minutes",
                        e.id, wf_name, e.status, elapsed_mins
                    ),
                    affected_entity: format!("execution:{}", e.id),
                    computed_at: now,
                });
            }
        }
    }

    findings
}

/// Detect retry storms (excessive retries on the same workflow).
fn detect_retry_storms(
    executions: &[ExecutionSummary],
    now: chrono::DateTime<Utc>,
) -> Vec<InsightFinding> {
    let mut findings = Vec::new();

    // Count retries per workflow
    let mut retries_by_workflow: HashMap<String, u32> = HashMap::new();
    let mut names: HashMap<String, String> = HashMap::new();

    for e in executions {
        if e.retry_of.is_some() {
            if let Some(ref wf_id) = e.workflow_id {
                *retries_by_workflow.entry(wf_id.clone()).or_default() += 1;
                if let Some(ref name) = e.workflow_name {
                    names.insert(wf_id.clone(), name.clone());
                }
            }
        }
    }

    for (wf_id, count) in &retries_by_workflow {
        if *count >= 5 {
            let wf_name = names.get(wf_id).cloned().unwrap_or_else(|| wf_id.clone());

            findings.push(InsightFinding {
                severity: Severity::Critical,
                category: InsightCategory::RetryStorm,
                title: format!("Retry storm: {} ({} retries)", wf_name, count),
                detail: format!(
                    "Workflow '{}' has {} retry executions in the current window",
                    wf_name, count
                ),
                affected_entity: format!("workflow:{}", wf_id),
                computed_at: now,
            });
        } else if *count >= 3 {
            let wf_name = names.get(wf_id).cloned().unwrap_or_else(|| wf_id.clone());

            findings.push(InsightFinding {
                severity: Severity::Warning,
                category: InsightCategory::RetryStorm,
                title: format!("Multiple retries: {} ({} retries)", wf_name, count),
                detail: format!(
                    "Workflow '{}' has {} retry executions in the current window",
                    wf_name, count
                ),
                affected_entity: format!("workflow:{}", wf_id),
                computed_at: now,
            });
        }
    }

    findings
}

/// Detect abnormally long-running executions.
fn detect_long_running(
    executions: &[ExecutionSummary],
    now: chrono::DateTime<Utc>,
) -> Vec<InsightFinding> {
    let mut findings = Vec::new();

    // Compute average duration per workflow
    let mut durations_by_workflow: HashMap<String, Vec<i64>> = HashMap::new();
    let mut names: HashMap<String, String> = HashMap::new();

    for e in executions {
        if let (Some(start), Some(stop), Some(ref wf_id)) =
            (e.started_at, e.stopped_at, &e.workflow_id)
        {
            let dur_ms = (stop - start).num_milliseconds();
            durations_by_workflow
                .entry(wf_id.clone())
                .or_default()
                .push(dur_ms);
            if let Some(ref name) = e.workflow_name {
                names.insert(wf_id.clone(), name.clone());
            }
        }
    }

    for (wf_id, durations) in &durations_by_workflow {
        if durations.len() < 3 {
            continue;
        }

        let avg: f64 = durations.iter().sum::<i64>() as f64 / durations.len() as f64;
        let max = *durations.iter().max().unwrap_or(&0);

        // Flag if any execution is >3x the average
        if max as f64 > avg * 3.0 && max > 60_000 {
            let wf_name = names.get(wf_id).cloned().unwrap_or_else(|| wf_id.clone());

            findings.push(InsightFinding {
                severity: Severity::Warning,
                category: InsightCategory::LongRunningExecution,
                title: format!("Long execution detected: {}", wf_name),
                detail: format!(
                    "Workflow '{}' has a max execution time of {:.1}s vs avg {:.1}s ({:.0}x slower)",
                    wf_name,
                    max as f64 / 1000.0,
                    avg / 1000.0,
                    max as f64 / avg
                ),
                affected_entity: format!("workflow:{}", wf_id),
                computed_at: now,
            });
        }
    }

    findings
}

/// Detect workflows that haven't been executed recently.
fn detect_abandoned_workflows(
    workflows: &[WorkflowSummary],
    executions: &[ExecutionSummary],
    now: chrono::DateTime<Utc>,
) -> Vec<InsightFinding> {
    let mut findings = Vec::new();

    // Build a set of workflow IDs that have recent executions
    let executed_wf_ids: std::collections::HashSet<String> = executions
        .iter()
        .filter_map(|e| e.workflow_id.clone())
        .collect();

    for wf in workflows {
        if !wf.active {
            continue; // Only flag active workflows with no executions
        }

        if !executed_wf_ids.contains(&wf.id) {
            if let Some(updated) = wf.updated_at {
                let days_since = (now - updated).num_days();
                if days_since > 30 {
                    findings.push(InsightFinding {
                        severity: Severity::Info,
                        category: InsightCategory::AbandonedWorkflow,
                        title: format!("Possibly abandoned: {}", wf.name),
                        detail: format!(
                            "Active workflow '{}' hasn't been updated in {} days and has no recent executions",
                            wf.name, days_since
                        ),
                        affected_entity: format!("workflow:{}", wf.id),
                        computed_at: now,
                    });
                }
            }
        }
    }

    findings
}

/// Detect inactive workflows tagged as critical/production.
fn detect_inactive_critical(
    workflows: &[WorkflowSummary],
    now: chrono::DateTime<Utc>,
) -> Vec<InsightFinding> {
    let mut findings = Vec::new();
    let critical_tags = ["production", "critical", "prod"];

    for wf in workflows {
        if wf.active {
            continue;
        }

        let has_critical_tag = wf
            .tags
            .iter()
            .any(|t| critical_tags.contains(&t.name.to_lowercase().as_str()));

        if has_critical_tag {
            findings.push(InsightFinding {
                severity: Severity::Warning,
                category: InsightCategory::InactiveCriticalWorkflow,
                title: format!("Critical workflow inactive: {}", wf.name),
                detail: format!(
                    "Workflow '{}' is tagged as production/critical but is currently inactive",
                    wf.name
                ),
                affected_entity: format!("workflow:{}", wf.id),
                computed_at: now,
            });
        }
    }

    findings
}
