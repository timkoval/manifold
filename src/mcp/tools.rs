//! MCP tool implementations

use crate::db::Database;
use crate::models::{Boundary, PatchEntry, SpecData, WorkflowStage};
use crate::workflow::WorkflowEngine;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::HashSet;

/// Valid top-level fields in SpecData (excluding read-only fields)
const VALID_MUTABLE_PATHS: &[&str] = &["name", "requirements", "tasks", "decisions"];

/// Valid fields for a Requirement object
const VALID_REQUIREMENT_FIELDS: &[&str] = &[
    "id",
    "capability",
    "title",
    "shall",
    "rationale",
    "priority",
    "tags",
    "scenarios",
];

/// Valid fields for a Scenario object
const VALID_SCENARIO_FIELDS: &[&str] = &["id", "name", "given", "when", "then", "edge_cases"];

/// Valid fields for a Task object
const VALID_TASK_FIELDS: &[&str] = &[
    "id",
    "requirement_ids",
    "title",
    "description",
    "status",
    "assignee",
    "acceptance",
];

/// Valid fields for a Decision object  
const VALID_DECISION_FIELDS: &[&str] = &[
    "id",
    "title",
    "context",
    "decision",
    "rationale",
    "alternatives_rejected",
    "date",
];

/// Validate that a patch path targets a known field
fn validate_patch_path(path: &str) -> Result<()> {
    // Parse path like "/requirements/-" or "/requirements/0/title"
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if parts.is_empty() || parts[0].is_empty() {
        bail!("Invalid patch path: '{}' - path cannot be empty", path);
    }

    let root = parts[0];

    // Check if root path is valid
    if !VALID_MUTABLE_PATHS.contains(&root) {
        bail!(
            "Invalid patch path: '{}' - '{}' is not a valid field. Valid fields are: {}",
            path,
            root,
            VALID_MUTABLE_PATHS.join(", ")
        );
    }

    Ok(())
}

/// Validate that an object being added/replaced has only valid fields
fn validate_object_fields(path: &str, value: &Value) -> Result<()> {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return Ok(()), // Not an object, nothing to validate
    };

    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if parts.is_empty() {
        return Ok(());
    }

    // Determine which schema to validate against based on path
    let valid_fields: &[&str] = match parts[0] {
        "requirements" => {
            // Check if we're adding a scenario inside a requirement
            if parts.len() >= 3 && parts.get(2) == Some(&"scenarios") {
                VALID_SCENARIO_FIELDS
            } else {
                VALID_REQUIREMENT_FIELDS
            }
        }
        "tasks" => VALID_TASK_FIELDS,
        "decisions" => VALID_DECISION_FIELDS,
        _ => return Ok(()), // Unknown path, already validated above
    };

    let valid_set: HashSet<&str> = valid_fields.iter().copied().collect();
    let mut invalid_fields = Vec::new();

    for key in obj.keys() {
        if !valid_set.contains(key.as_str()) {
            invalid_fields.push(key.clone());
        }
    }

    if !invalid_fields.is_empty() {
        bail!(
            "Invalid fields in patch value for '{}': [{}]. Valid fields are: {}",
            path,
            invalid_fields.join(", "),
            valid_fields.join(", ")
        );
    }

    Ok(())
}

/// Create a new spec
pub async fn create_spec(db: &mut Database, args: Value) -> Result<Value> {
    let project = args["project"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'project' parameter"))?;
    let boundary_str = args["boundary"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'boundary' parameter"))?;
    let name = args["name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'name' parameter"))?;

    // Parse boundary
    let boundary = match boundary_str {
        "personal" => Boundary::Personal,
        "work" => Boundary::Work,
        "company" => Boundary::Company,
        _ => bail!("Invalid boundary: must be 'personal', 'work', or 'company'"),
    };

    // Generate spec ID
    let spec_id = crate::db::generate_spec_id(project);

    // Create spec data
    let now = chrono::Utc::now().timestamp();
    let spec = SpecData {
        schema: "manifold://core/v1".to_string(),
        spec_id: spec_id.clone(),
        project: project.to_string(),
        boundary,
        name: name.to_string(),
        stage: WorkflowStage::Requirements,
        stages_completed: vec![],
        requirements: vec![],
        tasks: vec![],
        decisions: vec![],
        history: crate::models::History {
            created_at: now,
            updated_at: now,
            patches: vec![PatchEntry {
                timestamp: now,
                actor: "mcp".to_string(),
                op: "create".to_string(),
                path: "/".to_string(),
                summary: format!("Created via MCP: {}", name),
            }],
        },
    };

    // Insert into database
    db.insert_spec(&spec)?;

    Ok(json!({
        "success": true,
        "spec_id": spec_id,
        "message": format!("Created spec '{}' in {} boundary", name, boundary_str)
    }))
}

/// Apply a JSON patch to a spec
pub async fn apply_patch(db: &mut Database, args: Value) -> Result<Value> {
    let spec_id = args["spec_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'spec_id' parameter"))?;
    let patch_ops = args["patch"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'patch' parameter"))?;
    let summary = args["summary"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'summary' parameter"))?;

    // Validate all patch operations before applying
    for (i, op) in patch_ops.iter().enumerate() {
        let path = op["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Patch operation {} missing 'path'", i))?;

        let op_type = op["op"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Patch operation {} missing 'op'", i))?;

        // Validate the path targets a known field
        validate_patch_path(path)?;

        // For add/replace operations, validate the value has valid fields
        if op_type == "add" || op_type == "replace" {
            if let Some(value) = op.get("value") {
                validate_object_fields(path, value)?;
            }
        }
    }

    // Get current spec
    let spec_row = db
        .get_spec(spec_id)?
        .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;
    let mut spec: SpecData = serde_json::from_value(spec_row.data.clone())?;

    // Convert to JSON for patching
    let mut spec_json = serde_json::to_value(&spec)?;

    // Apply patch operations - convert Vec<Value> to Patch
    let patch_value = serde_json::Value::Array(patch_ops.clone());
    let patch: json_patch::Patch = serde_json::from_value(patch_value)?;
    json_patch::patch(&mut spec_json, &patch)
        .map_err(|e| anyhow::anyhow!("Failed to apply patch: {}", e))?;

    // Convert back to SpecData
    spec = serde_json::from_value(spec_json.clone())?;

    // Update history
    let now = chrono::Utc::now().timestamp();
    spec.history.updated_at = now;
    spec.history.patches.push(PatchEntry {
        timestamp: now,
        actor: "mcp".to_string(),
        op: "patch".to_string(),
        path: "/".to_string(),
        summary: summary.to_string(),
    });

    // Update in database
    db.update_spec(&spec)?;

    Ok(json!({
        "success": true,
        "spec_id": spec_id,
        "message": format!("Applied patch: {}", summary)
    }))
}

/// Advance a spec to a new workflow stage
pub async fn advance_workflow(db: &mut Database, args: Value) -> Result<Value> {
    let spec_id = args["spec_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'spec_id' parameter"))?;
    let target_stage_str = args["target_stage"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'target_stage' parameter"))?;

    // Parse target stage
    let target_stage = match target_stage_str {
        "requirements" => WorkflowStage::Requirements,
        "design" => WorkflowStage::Design,
        "tasks" => WorkflowStage::Tasks,
        "approval" => WorkflowStage::Approval,
        "implemented" => WorkflowStage::Implemented,
        _ => bail!("Invalid stage: {}", target_stage_str),
    };

    // Get current spec
    let spec_row = db
        .get_spec(spec_id)?
        .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;
    let mut spec: SpecData = serde_json::from_value(spec_row.data)?;

    // Validate and execute transition using workflow engine
    match WorkflowEngine::advance_stage(&spec, target_stage) {
        Ok(transition) => {
            // Update spec
            let old_stage = spec.stage.clone();
            if !spec.stages_completed.contains(&old_stage) {
                spec.stages_completed.push(old_stage.clone());
            }
            spec.stage = transition.to.clone();

            // Update history
            let now = chrono::Utc::now().timestamp();
            spec.history.updated_at = now;
            spec.history.patches.push(PatchEntry {
                timestamp: now,
                actor: "mcp".to_string(),
                op: "advance".to_string(),
                path: "/stage".to_string(),
                summary: format!("Advanced from {} to {}", transition.from, transition.to),
            });

            // Log workflow event
            db.log_workflow_event(
                spec_id,
                &transition.to.to_string(),
                &transition.event.as_string(),
                "mcp",
                now,
                Some(&format!(
                    "Advanced from {} to {}",
                    transition.from, transition.to
                )),
            )?;

            // Update in database
            db.update_spec(&spec)?;

            Ok(json!({
                "success": true,
                "spec_id": spec_id,
                "old_stage": transition.from.to_string(),
                "new_stage": transition.to.to_string(),
                "stages_completed": spec.stages_completed.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                "message": format!("Advanced to {} stage", transition.to)
            }))
        }
        Err(e) => {
            // Log failed validation
            db.log_workflow_event(
                spec_id,
                &spec.stage.to_string(),
                &format!("validation_failed:{}", e),
                "mcp",
                chrono::Utc::now().timestamp(),
                Some(&e.to_string()),
            )?;

            Ok(json!({
                "success": false,
                "spec_id": spec_id,
                "current_stage": spec.stage.to_string(),
                "error": e.to_string(),
                "message": format!("Cannot advance: {}", e)
            }))
        }
    }
}

/// Query/search specs in manifold
pub async fn query_manifold(db: &Database, args: Value) -> Result<Value> {
    let boundary_filter = args.get("boundary").and_then(|v| v.as_str());
    let stage_filter = args.get("stage").and_then(|v| v.as_str());
    let _project_filter = args.get("project").and_then(|v| v.as_str());

    // Parse filters
    let boundary_enum = boundary_filter.and_then(|b| match b {
        "personal" => Some(Boundary::Personal),
        "work" => Some(Boundary::Work),
        "company" => Some(Boundary::Company),
        _ => None,
    });

    let stage_enum = stage_filter.and_then(|s| match s {
        "requirements" => Some(WorkflowStage::Requirements),
        "design" => Some(WorkflowStage::Design),
        "tasks" => Some(WorkflowStage::Tasks),
        "approval" => Some(WorkflowStage::Approval),
        "implemented" => Some(WorkflowStage::Implemented),
        _ => None,
    });

    // Get specs with filters (boundary and stage handled by DB query)
    let filtered_specs = db.list_specs(boundary_enum.as_ref(), stage_enum.as_ref())?;

    // Convert to JSON
    let results: Vec<Value> = filtered_specs
        .iter()
        .map(|spec| {
            // Parse the data to get the name
            let spec_data: Result<SpecData, _> = serde_json::from_value(spec.data.clone());
            let name = spec_data
                .map(|s| s.name)
                .unwrap_or_else(|_| "Unknown".to_string());

            json!({
                "spec_id": spec.id,
                "project": spec.project,
                "name": name,
                "boundary": spec.boundary,
                "stage": spec.stage,
                "updated_at": spec.updated_at
            })
        })
        .collect();

    Ok(json!({
        "success": true,
        "count": results.len(),
        "specs": results
    }))
}
