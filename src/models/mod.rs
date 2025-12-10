//! Data models for manifold specs
//!
//! These represent the canonical JSON structure stored in SQLite
//! Optimized for LLM consumption and production

use serde::{Deserialize, Serialize};

/// Boundary type for spec isolation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Boundary {
    Personal,
    Work,
    Company,
}

impl std::fmt::Display for Boundary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Boundary::Personal => write!(f, "personal"),
            Boundary::Work => write!(f, "work"),
            Boundary::Company => write!(f, "company"),
        }
    }
}

impl std::str::FromStr for Boundary {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "personal" => Ok(Boundary::Personal),
            "work" => Ok(Boundary::Work),
            "company" => Ok(Boundary::Company),
            _ => Err(format!("Invalid boundary: {}. Use: personal, work, company", s)),
        }
    }
}

/// Workflow stages for a spec
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStage {
    Requirements,
    Design,
    Tasks,
    Approval,
    Implemented,
}

impl std::fmt::Display for WorkflowStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStage::Requirements => write!(f, "requirements"),
            WorkflowStage::Design => write!(f, "design"),
            WorkflowStage::Tasks => write!(f, "tasks"),
            WorkflowStage::Approval => write!(f, "approval"),
            WorkflowStage::Implemented => write!(f, "implemented"),
        }
    }
}

impl std::str::FromStr for WorkflowStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "requirements" => Ok(WorkflowStage::Requirements),
            "design" => Ok(WorkflowStage::Design),
            "tasks" => Ok(WorkflowStage::Tasks),
            "approval" => Ok(WorkflowStage::Approval),
            "implemented" => Ok(WorkflowStage::Implemented),
            _ => Err(format!(
                "Invalid stage: {}. Use: requirements, design, tasks, approval, implemented",
                s
            )),
        }
    }
}

/// Priority level (MoSCoW)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Must,
    Should,
    Could,
    Wont,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Must => write!(f, "must"),
            Priority::Should => write!(f, "should"),
            Priority::Could => write!(f, "could"),
            Priority::Wont => write!(f, "wont"),
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Should
    }
}

/// A scenario using GIVEN/WHEN/THEN pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub given: Vec<String>,
    pub when: String,
    pub then: Vec<String>,
    #[serde(default)]
    pub edge_cases: Vec<String>,
}

/// A requirement with SHALL statement and scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub capability: String,
    pub title: String,
    pub shall: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Blocked => write!(f, "blocked"),
        }
    }
}

/// A task with explicit requirement traceability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub requirement_ids: Vec<String>,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default)]
    pub acceptance: Vec<String>,
}

/// A design decision with rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub title: String,
    pub context: String,
    pub decision: String,
    pub rationale: String,
    #[serde(default)]
    pub alternatives_rejected: Vec<String>,
    pub date: String,
}

/// Patch history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchEntry {
    pub timestamp: i64,
    pub actor: String,
    pub op: String,
    pub path: String,
    pub summary: String,
}

/// History and change tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub patches: Vec<PatchEntry>,
}

/// The full canonical spec document (LLM-native)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecData {
    #[serde(rename = "$schema", default = "default_schema")]
    pub schema: String,
    
    pub spec_id: String,
    pub project: String,
    pub boundary: Boundary,
    pub name: String,
    
    pub stage: WorkflowStage,
    #[serde(default)]
    pub stages_completed: Vec<WorkflowStage>,
    
    #[serde(default)]
    pub requirements: Vec<Requirement>,
    
    #[serde(default)]
    pub tasks: Vec<Task>,
    
    #[serde(default)]
    pub decisions: Vec<Decision>,
    
    pub history: History,
}

fn default_schema() -> String {
    "manifold://core/v1".to_string()
}

impl SpecData {
    /// Create a new spec with minimal required fields
    pub fn new(spec_id: String, project: String, name: String, boundary: Boundary) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            schema: default_schema(),
            spec_id,
            project,
            name,
            boundary,
            stage: WorkflowStage::Requirements,
            stages_completed: Vec::new(),
            requirements: Vec::new(),
            tasks: Vec::new(),
            decisions: Vec::new(),
            history: History {
                created_at: now,
                updated_at: now,
                patches: Vec::new(),
            },
        }
    }
    
    /// Get the current workflow stage
    pub fn current_stage(&self) -> &WorkflowStage {
        &self.stage
    }
    
    /// Get requirement by ID
    pub fn get_requirement(&self, id: &str) -> Option<&Requirement> {
        self.requirements.iter().find(|r| r.id == id)
    }
    
    /// Get task by ID
    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }
}

/// Database row representation of a spec
#[derive(Debug, Clone)]
pub struct SpecRow {
    pub id: String,
    pub project: String,
    pub boundary: String,
    pub data: serde_json::Value,
    pub stage: String,
    pub updated_at: i64,
    pub created_at: i64,
}
