//! Multi-step task planner tools.
//!
//! Provides `plan` and `plan_update` tools that let the agent declare a
//! structured plan before executing, and report progress as steps complete.
//! The plan is rendered as a checklist visible to channel users.

use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Types ────────────────────────────────────────────────────────

/// Status of a single plan step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
    Failed,
}

impl StepStatus {
    fn emoji(self) -> &'static str {
        match self {
            Self::Pending => "\u{2b1c}",     // ⬜
            Self::InProgress => "\u{1f504}", // 🔄
            Self::Completed => "\u{2705}",   // ✅
            Self::Skipped => "\u{23ed}",     // ⏭
            Self::Failed => "\u{274c}",      // ❌
        }
    }
}

/// A single step in a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub title: String,
    pub description: String,
    pub status: StepStatus,
}

/// A multi-step task plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub steps: Vec<PlanStep>,
    pub created_at: String,
}

impl TaskPlan {
    /// Render the plan as a plain-text checklist suitable for messaging channels.
    pub fn format_checklist(&self) -> String {
        let mut out = String::from("\u{1f4cb} Plan:\n"); // 📋
        for (i, step) in self.steps.iter().enumerate() {
            let marker = if step.status == StepStatus::InProgress {
                format!("{} {}  \u{2190} current", step.status.emoji(), step.title) // ←
            } else {
                format!("{} {}", step.status.emoji(), step.title)
            };
            out.push_str(&format!("{}. {}\n", i + 1, marker));
        }
        out
    }
}

// ── Store ────────────────────────────────────────────────────────

/// Per-session in-memory store for the active plan.
pub struct TaskPlanStore {
    current_plan: Mutex<Option<TaskPlan>>,
}

impl TaskPlanStore {
    pub fn new() -> Self {
        Self {
            current_plan: Mutex::new(None),
        }
    }

    pub fn set_plan(&self, plan: TaskPlan) {
        *self.current_plan.lock() = Some(plan);
    }

    pub fn update_step(&self, step_index: usize, status: StepStatus) -> Option<TaskPlan> {
        let mut guard = self.current_plan.lock();
        if let Some(ref mut plan) = *guard {
            if step_index < plan.steps.len() {
                plan.steps[step_index].status = status;
            }
            Some(plan.clone())
        } else {
            None
        }
    }

    pub fn current_plan(&self) -> Option<TaskPlan> {
        self.current_plan.lock().clone()
    }
}

// ── PlanTool ─────────────────────────────────────────────────────

/// Tool that lets the agent declare a multi-step plan before executing.
pub struct PlanTool {
    store: Arc<TaskPlanStore>,
}

impl PlanTool {
    pub fn new(store: Arc<TaskPlanStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for PlanTool {
    fn name(&self) -> &str {
        "plan"
    }

    fn description(&self) -> &str {
        "Declare a multi-step plan before executing a complex task. \
         Call this at the start of tasks that require 3 or more steps \
         so the user can see your approach. Each step should have a \
         short title and optional description."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["steps"],
            "properties": {
                "steps": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["title"],
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "Short title for the step"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional longer description of what this step does",
                                "default": ""
                            }
                        }
                    },
                    "description": "Ordered list of steps in the plan"
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let steps_val = args
            .get("steps")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("missing 'steps' array"))?;

        let mut steps = Vec::with_capacity(steps_val.len());
        for s in steps_val {
            let title = s
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();
            let description = s
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            steps.push(PlanStep {
                title,
                description,
                status: StepStatus::Pending,
            });
        }

        let plan = TaskPlan {
            steps,
            created_at: Utc::now().to_rfc3339(),
        };

        let checklist = plan.format_checklist();
        self.store.set_plan(plan);

        Ok(ToolResult {
            success: true,
            output: checklist,
            error: None,
        })
    }
}

// ── PlanUpdateTool ───────────────────────────────────────────────

/// Tool that updates the status of a step in the current plan.
pub struct PlanUpdateTool {
    store: Arc<TaskPlanStore>,
}

impl PlanUpdateTool {
    pub fn new(store: Arc<TaskPlanStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for PlanUpdateTool {
    fn name(&self) -> &str {
        "plan_update"
    }

    fn description(&self) -> &str {
        "Update the status of a step in the current plan. \
         Call this as you begin or finish each step so the user \
         can track progress. Use step index (0-based)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["step", "status"],
            "properties": {
                "step": {
                    "type": "integer",
                    "description": "Zero-based index of the step to update"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "in_progress", "completed", "skipped", "failed"],
                    "description": "New status for the step"
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let step_index = args
            .get("step")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("missing 'step' integer"))? as usize;

        let status_str = args
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'status' string"))?;

        let status: StepStatus = serde_json::from_value(serde_json::Value::String(
            status_str.to_string(),
        ))
        .map_err(|_| {
            anyhow::anyhow!(
                "invalid status '{}': expected pending, in_progress, completed, skipped, or failed",
                status_str
            )
        })?;

        match self.store.update_step(step_index, status) {
            Some(plan) => Ok(ToolResult {
                success: true,
                output: plan.format_checklist(),
                error: None,
            }),
            None => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("No active plan. Call 'plan' first.".to_string()),
            }),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_status_emoji() {
        assert_eq!(StepStatus::Pending.emoji(), "\u{2b1c}");
        assert_eq!(StepStatus::InProgress.emoji(), "\u{1f504}");
        assert_eq!(StepStatus::Completed.emoji(), "\u{2705}");
    }

    #[test]
    fn plan_format_checklist() {
        let plan = TaskPlan {
            steps: vec![
                PlanStep {
                    title: "Check status".into(),
                    description: String::new(),
                    status: StepStatus::Completed,
                },
                PlanStep {
                    title: "Run tests".into(),
                    description: String::new(),
                    status: StepStatus::InProgress,
                },
                PlanStep {
                    title: "Deploy".into(),
                    description: String::new(),
                    status: StepStatus::Pending,
                },
            ],
            created_at: "2026-03-27T00:00:00Z".into(),
        };
        let checklist = plan.format_checklist();
        assert!(checklist.contains("Check status"));
        assert!(checklist.contains("Run tests"));
        assert!(checklist.contains("Deploy"));
        assert!(checklist.contains("\u{2190} current")); // ← current marker
    }

    #[test]
    fn store_set_and_update() {
        let store = TaskPlanStore::new();
        assert!(store.current_plan().is_none());

        let plan = TaskPlan {
            steps: vec![
                PlanStep {
                    title: "Step A".into(),
                    description: String::new(),
                    status: StepStatus::Pending,
                },
                PlanStep {
                    title: "Step B".into(),
                    description: String::new(),
                    status: StepStatus::Pending,
                },
            ],
            created_at: "2026-03-27T00:00:00Z".into(),
        };
        store.set_plan(plan);
        assert!(store.current_plan().is_some());

        let updated = store.update_step(0, StepStatus::Completed);
        assert!(updated.is_some());
        let p = updated.unwrap();
        assert_eq!(p.steps[0].status, StepStatus::Completed);
        assert_eq!(p.steps[1].status, StepStatus::Pending);
    }

    #[test]
    fn store_update_no_plan_returns_none() {
        let store = TaskPlanStore::new();
        assert!(store.update_step(0, StepStatus::Completed).is_none());
    }

    #[tokio::test]
    async fn plan_tool_execute() {
        let store = Arc::new(TaskPlanStore::new());
        let tool = PlanTool::new(Arc::clone(&store));

        let args = serde_json::json!({
            "steps": [
                {"title": "Analyze code", "description": "Read files"},
                {"title": "Make changes"},
                {"title": "Run tests", "description": "cargo test"}
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Analyze code"));
        assert!(result.output.contains("Make changes"));
        assert!(result.output.contains("Run tests"));

        let plan = store.current_plan().unwrap();
        assert_eq!(plan.steps.len(), 3);
        assert!(plan.steps.iter().all(|s| s.status == StepStatus::Pending));
    }

    #[tokio::test]
    async fn plan_update_tool_execute() {
        let store = Arc::new(TaskPlanStore::new());
        let plan_tool = PlanTool::new(Arc::clone(&store));
        let update_tool = PlanUpdateTool::new(Arc::clone(&store));

        // Create a plan first.
        let args = serde_json::json!({
            "steps": [
                {"title": "Step 1"},
                {"title": "Step 2"}
            ]
        });
        plan_tool.execute(args).await.unwrap();

        // Update step 0 to completed.
        let result = update_tool
            .execute(serde_json::json!({"step": 0, "status": "completed"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("\u{2705}")); // ✅

        // Update step 1 to in_progress.
        let result = update_tool
            .execute(serde_json::json!({"step": 1, "status": "in_progress"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("\u{2190} current"));
    }

    #[tokio::test]
    async fn plan_update_no_plan_fails() {
        let store = Arc::new(TaskPlanStore::new());
        let tool = PlanUpdateTool::new(store);

        let result = tool
            .execute(serde_json::json!({"step": 0, "status": "completed"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("No active plan"));
    }
}
