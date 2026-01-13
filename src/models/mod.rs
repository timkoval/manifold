//! Data models for manifold specs
//!
//! These represent the canonical JSON structure stored in SQLite
//! Optimized for LLM consumption and production

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Boundary type for spec isolation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Boundary {
    Personal,
    Work,
    Company,
}

/// Extended boundary visibility for v2 manifold
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BoundaryVisibility {
    Public,
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
            _ => Err(format!(
                "Invalid boundary: {}. Use: personal, work, company",
                s
            )),
        }
    }
}

impl std::fmt::Display for BoundaryVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoundaryVisibility::Public => write!(f, "public"),
            BoundaryVisibility::Personal => write!(f, "personal"),
            BoundaryVisibility::Work => write!(f, "work"),
            BoundaryVisibility::Company => write!(f, "company"),
        }
    }
}

impl std::str::FromStr for BoundaryVisibility {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "public" => Ok(BoundaryVisibility::Public),
            "personal" => Ok(BoundaryVisibility::Personal),
            "work" => Ok(BoundaryVisibility::Work),
            "company" => Ok(BoundaryVisibility::Company),
            _ => Err(format!(
                "Invalid visibility: {}. Use: public, personal, work, company",
                s
            )),
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
    /// Utility method for external consumers of the library
    #[allow(dead_code)]
    pub fn current_stage(&self) -> &WorkflowStage {
        &self.stage
    }

    /// Get requirement by ID
    /// Utility method for external consumers of the library
    #[allow(dead_code)]
    pub fn get_requirement(&self, id: &str) -> Option<&Requirement> {
        self.requirements.iter().find(|r| r.id == id)
    }

    /// Get task by ID
    /// Utility method for external consumers of the library
    #[allow(dead_code)]
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

// ============================================================================
// V2 Manifold Structures
// ============================================================================

/// Manifold v2 - The top-level document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifoldV2 {
    #[serde(rename = "$schema", default = "default_schema_v2")]
    pub schema: String,

    pub manifold_id: String,
    pub boundaries: std::collections::HashMap<String, BoundaryConfig>,

    pub nodes: Vec<Node>,

    #[serde(default)]
    pub version: i32,
}

pub fn default_schema_v2() -> String {
    "manifold://core/v2".to_string()
}

/// Boundary configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryConfig {
    pub visibility: BoundaryVisibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Node type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Project,
    Spec,
    Knowledge,
    Diary,
    Research,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Project => write!(f, "project"),
            NodeType::Spec => write!(f, "spec"),
            NodeType::Knowledge => write!(f, "knowledge"),
            NodeType::Diary => write!(f, "diary"),
            NodeType::Research => write!(f, "research"),
        }
    }
}

impl std::str::FromStr for NodeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "project" => Ok(NodeType::Project),
            "spec" => Ok(NodeType::Spec),
            "knowledge" => Ok(NodeType::Knowledge),
            "diary" => Ok(NodeType::Diary),
            "research" => Ok(NodeType::Research),
            _ => Err(format!(
                "Invalid node type: {}. Use: project, spec, knowledge, diary, research",
                s
            )),
        }
    }
}

/// A node in the manifold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub boundary: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<NodeContent>,

    #[serde(default)]
    pub links: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<History>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<std::collections::HashMap<String, String>>,
}

/// Node content - varies by type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodeContent {
    Project(ProjectContent),
    Spec(SpecContent),
    Knowledge(KnowledgeContent),
    Diary(DiaryContent),
    Research(ResearchContent),
}

/// Project node content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    pub requirements: Vec<Requirement>,

    #[serde(default)]
    pub scenarios: Vec<Scenario>,

    #[serde(default)]
    pub tasks: Vec<Task>,

    #[serde(default)]
    pub decisions: Vec<Decision>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_stage: Option<String>,
}

/// Spec node content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecContent {
    pub project_id: String,

    #[serde(default)]
    pub requirements: Vec<Requirement>,

    #[serde(default)]
    pub tasks: Vec<Task>,

    #[serde(default)]
    pub decisions: Vec<Decision>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_stage: Option<String>,

    #[serde(default)]
    pub stages_completed: Vec<String>,
}

/// Knowledge node content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeContent {
    pub topic: String,
    pub notes: String,

    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(default)]
    pub linked_specs: Vec<String>,
}

/// Diary node content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryContent {
    pub date: String,
    pub reflection: String,

    #[serde(default)]
    pub linked_specs: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
}

/// Research node content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchContent {
    pub hub: String,
    pub entries: Vec<ResearchEntry>,

    #[serde(default)]
    pub linked_specs: Vec<String>,
}

/// Research entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchEntry {
    pub source: String,
    pub summary: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

// ============================================================================
// Database row for v2 nodes
// ============================================================================

/// Database row representation of a v2 node
#[derive(Debug, Clone)]
pub struct NodeRow {
    pub id: String,
    pub manifold_id: String,
    pub node_type: String,
    pub boundary: String,
    pub title: Option<String>,
    pub content: Option<Value>,
    pub links: Vec<String>,
    pub updated_at: Option<i64>,
    pub created_at: Option<i64>,
}
