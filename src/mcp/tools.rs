//! MCP tool implementations

use anyhow::{Result, bail};
use serde_json::{json, Value};
use crate::db::Database;
use crate::models::{SpecData, WorkflowStage, Boundary, PatchEntry};

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

    // Get current spec
    let spec_row = db.get_spec(spec_id)?
        .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;
    let mut spec: SpecData = serde_json::from_value(spec_row.data)?;

    // Convert to JSON for patching
    let mut spec_json = serde_json::to_value(&spec)?;

    // Apply patch operations - convert Vec<Value> to Patch
    let patch_value = serde_json::Value::Array(patch_ops.clone());
    let patch: json_patch::Patch = serde_json::from_value(patch_value)?;
    json_patch::patch(&mut spec_json, &patch)
        .map_err(|e| anyhow::anyhow!("Failed to apply patch: {}", e))?;

    // Convert back to SpecData
    spec = serde_json::from_value(spec_json)?;

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
    let spec_row = db.get_spec(spec_id)?
        .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;
    let mut spec: SpecData = serde_json::from_value(spec_row.data)?;

    // Add current stage to completed stages if not already there
    if !spec.stages_completed.contains(&spec.stage) {
        spec.stages_completed.push(spec.stage.clone());
    }

    let old_stage = spec.stage.clone();
    spec.stage = target_stage.clone();

    // Update history
    let now = chrono::Utc::now().timestamp();
    spec.history.updated_at = now;
    spec.history.patches.push(PatchEntry {
        timestamp: now,
        actor: "mcp".to_string(),
        op: "advance".to_string(),
        path: "/stage".to_string(),
        summary: format!("Advanced from {:?} to {:?}", old_stage, target_stage),
    });

    // Update in database
    db.update_spec(&spec)?;

    Ok(json!({
        "success": true,
        "spec_id": spec_id,
        "old_stage": format!("{:?}", old_stage),
        "new_stage": format!("{:?}", target_stage),
        "message": format!("Advanced to {:?} stage", target_stage)
    }))
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
            let name = spec_data.map(|s| s.name).unwrap_or_else(|_| "Unknown".to_string());
            
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
