//! Schema validation for manifold specs

use crate::models::{BoundaryConfig, ManifoldV2, Node, SpecData};
use anyhow::{bail, Context, Result};
use jsonschema::JSONSchema;
use serde_json::Value;

/// Validate a spec against the JSON schema
pub fn validate_spec(spec: &SpecData) -> Result<()> {
    // First validate against JSON schema
    validate_against_schema(spec)?;

    // Then run basic structural checks
    // Check required fields
    if spec.spec_id.is_empty() {
        bail!("spec_id is required");
    }
    if spec.project.is_empty() {
        bail!("project is required");
    }
    if spec.name.is_empty() {
        bail!("name is required");
    }

    // Validate IDs follow patterns
    validate_id_pattern(&spec.spec_id, "spec_id", r"^[a-z][a-z0-9-]*$")?;
    validate_id_pattern(&spec.project, "project", r"^[a-z][a-z0-9-]*$")?;

    // Validate requirements
    for req in &spec.requirements {
        validate_id_pattern(&req.id, "requirement id", r"^req-[0-9]+$")?;
        if req.title.is_empty() {
            bail!("Requirement {} has empty title", req.id);
        }
        if req.shall.is_empty() {
            bail!("Requirement {} has empty 'shall' statement", req.id);
        }

        // Validate scenarios
        for scenario in &req.scenarios {
            validate_id_pattern(&scenario.id, "scenario id", r"^sc-[0-9]+$")?;
            if scenario.name.is_empty() {
                bail!("Scenario {} has empty name", scenario.id);
            }
        }
    }

    // Validate tasks
    for task in &spec.tasks {
        validate_id_pattern(&task.id, "task id", r"^task-[0-9]+$")?;
        if task.title.is_empty() {
            bail!("Task {} has empty title", task.id);
        }
    }

    // Validate decisions
    for decision in &spec.decisions {
        validate_id_pattern(&decision.id, "decision id", r"^dec-[0-9]+$")?;
        if decision.title.is_empty() {
            bail!("Decision {} has empty title", decision.id);
        }
    }

    Ok(())
}

fn validate_id_pattern(id: &str, name: &str, pattern: &str) -> Result<()> {
    let re = regex::Regex::new(pattern).context("Invalid regex pattern in validation")?;
    if !re.is_match(id) {
        bail!(
            "{} '{}' doesn't match required pattern {}",
            name,
            id,
            pattern
        );
    }
    Ok(())
}

/// Validate spec against JSON schema
fn validate_against_schema(spec: &SpecData) -> Result<()> {
    // Load the schema
    let schema_path = crate::config::manifold_home()?.join("schemas/core.json");
    let schema_content = std::fs::read_to_string(&schema_path)
        .map_err(|e| anyhow::anyhow!("Failed to read schema from {:?}: {}", schema_path, e))?;
    let schema_json: Value = serde_json::from_str(&schema_content)?;

    // Compile the schema
    let compiled = JSONSchema::compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))?;

    // Convert spec to JSON
    let spec_json = serde_json::to_value(spec)?;

    // Validate
    if let Err(errors) = compiled.validate(&spec_json) {
        let error_messages: Vec<String> = errors.map(|e| format!("{}", e)).collect();
        bail!("Schema validation failed:\n{}", error_messages.join("\n"));
    }

    Ok(())
}

/// Check for common spec issues (lint-like checks)
pub fn lint_spec(spec: &SpecData) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check for empty requirements
    if spec.requirements.is_empty() {
        warnings.push("Spec has no requirements defined".to_string());
    }

    // Check each requirement
    for req in &spec.requirements {
        // Requirements should have at least one scenario
        if req.scenarios.is_empty() {
            warnings.push(format!("{}: No scenarios defined", req.id));
        }

        // Check for SHALL/MUST in requirement statement
        let shall_upper = req.shall.to_uppercase();
        if !shall_upper.contains("SHALL") && !shall_upper.contains("MUST") {
            warnings.push(format!("{}: Requirement doesn't use SHALL or MUST", req.id));
        }

        // Scenarios should have non-empty given/then
        for scenario in &req.scenarios {
            if scenario.given.is_empty() {
                warnings.push(format!(
                    "{}/{}: Empty 'given' preconditions",
                    req.id, scenario.id
                ));
            }
            if scenario.then.is_empty() {
                warnings.push(format!("{}/{}: Empty 'then' outcomes", req.id, scenario.id));
            }
        }
    }

    // Check tasks
    for task in &spec.tasks {
        // Tasks should reference at least one requirement
        if task.requirement_ids.is_empty() {
            warnings.push(format!(
                "{}: Task doesn't reference any requirements",
                task.id
            ));
        }

        // Check that referenced requirements exist
        for req_id in &task.requirement_ids {
            if !spec.requirements.iter().any(|r| &r.id == req_id) {
                warnings.push(format!(
                    "{}: References non-existent requirement {}",
                    task.id, req_id
                ));
            }
        }

        // Tasks should have acceptance criteria
        if task.acceptance.is_empty() {
            warnings.push(format!("{}: No acceptance criteria defined", task.id));
        }
    }

    // Check for duplicate IDs
    let mut req_ids = std::collections::HashSet::new();
    for req in &spec.requirements {
        if !req_ids.insert(&req.id) {
            warnings.push(format!("Duplicate requirement ID: {}", req.id));
        }
    }

    let mut task_ids = std::collections::HashSet::new();
    for task in &spec.tasks {
        if !task_ids.insert(&task.id) {
            warnings.push(format!("Duplicate task ID: {}", task.id));
        }
    }

    warnings
}

// ============================================================================
// V2 Manifold Validation
// ============================================================================

/// Validate a manifold against v2 schema
pub fn validate_manifold(manifold: &ManifoldV2) -> Result<()> {
    // Validate against JSON schema
    validate_against_schema_v2(manifold)?;

    // Structural checks
    if manifold.manifold_id.is_empty() {
        bail!("manifold_id is required");
    }

    validate_id_pattern(&manifold.manifold_id, "manifold_id", r"^[a-z][a-z0-9-]*$")?;

    // Validate boundaries exist
    if manifold.boundaries.is_empty() {
        bail!("At least one boundary must be defined");
    }

    // Validate nodes
    for node in &manifold.nodes {
        validate_node(node, &manifold.boundaries)?;
    }

    Ok(())
}

/// Validate a node against boundaries and type-specific rules
pub fn validate_node(
    node: &Node,
    boundaries: &std::collections::HashMap<String, BoundaryConfig>,
) -> Result<()> {
    // Check required fields
    if node.id.is_empty() {
        bail!("node id is required");
    }
    if node.boundary.is_empty() {
        bail!("node boundary is required");
    }

    // Validate ID pattern
    validate_id_pattern(&node.id, "node id", r"^[a-z][a-z0-9-]*$")?;

    // Validate boundary exists in manifold
    if !boundaries.contains_key(&node.boundary) {
        bail!(
            "Boundary '{}' is not defined in manifold. Available: {:?}",
            node.boundary,
            boundaries.keys().collect::<Vec<_>>()
        );
    }

    // Validate node type-specific content
    if let Some(content) = &node.content {
        match content {
            crate::models::NodeContent::Project(proj) => {
                for req in &proj.requirements {
                    validate_id_pattern(&req.id, "requirement id", r"^req-[0-9]+$")?;
                    if req.title.is_empty() {
                        bail!(
                            "Requirement {} in project node {} has empty title",
                            req.id,
                            node.id
                        );
                    }
                    if req.shall.is_empty() {
                        bail!(
                            "Requirement {} in project node {} has empty 'shall' statement",
                            req.id,
                            node.id
                        );
                    }
                }
                for task in &proj.tasks {
                    validate_id_pattern(&task.id, "task id", r"^task-[0-9]+$")?;
                    if task.title.is_empty() {
                        bail!(
                            "Task {} in project node {} has empty title",
                            task.id,
                            node.id
                        );
                    }
                }
            }
            crate::models::NodeContent::Spec(spec) => {
                for req in &spec.requirements {
                    validate_id_pattern(&req.id, "requirement id", r"^req-[0-9]+$")?;
                    if req.title.is_empty() {
                        bail!(
                            "Requirement {} in spec node {} has empty title",
                            req.id,
                            node.id
                        );
                    }
                    if req.shall.is_empty() {
                        bail!(
                            "Requirement {} in spec node {} has empty 'shall' statement",
                            req.id,
                            node.id
                        );
                    }
                }
                for task in &spec.tasks {
                    validate_id_pattern(&task.id, "task id", r"^task-[0-9]+$")?;
                    if task.title.is_empty() {
                        bail!("Task {} in spec node {} has empty title", task.id, node.id);
                    }
                }
            }
            crate::models::NodeContent::Knowledge(k) => {
                if k.topic.is_empty() {
                    bail!("Knowledge node {} has empty topic", node.id);
                }
                if k.notes.is_empty() {
                    bail!("Knowledge node {} has empty notes", node.id);
                }
            }
            crate::models::NodeContent::Diary(d) => {
                if d.date.is_empty() {
                    bail!("Diary node {} has empty date", node.id);
                }
                if d.reflection.is_empty() {
                    bail!("Diary node {} has empty reflection", node.id);
                }
                // Validate date format
                chrono::NaiveDate::parse_from_str(&d.date, "%Y-%m-%d").map_err(|_| {
                    anyhow::anyhow!(
                        "Invalid date format in diary node {}: expected YYYY-MM-DD",
                        node.id
                    )
                })?;
            }
            crate::models::NodeContent::Research(r) => {
                if r.hub.is_empty() {
                    bail!("Research node {} has empty hub", node.id);
                }
                if r.entries.is_empty() {
                    bail!("Research node {} has no entries", node.id);
                }
                for entry in &r.entries {
                    if entry.source.is_empty() {
                        bail!("Research entry in node {} has empty source", node.id);
                    }
                    if entry.summary.is_empty() {
                        bail!("Research entry in node {} has empty summary", node.id);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Validate spec against JSON schema (v1)
fn validate_against_schema_v1(spec: &SpecData) -> Result<()> {
    // Load schema
    let schema_path = crate::config::manifold_home()?.join("schemas/core.json");
    let schema_content = std::fs::read_to_string(&schema_path)
        .map_err(|e| anyhow::anyhow!("Failed to read schema from {:?}: {}", schema_path, e))?;
    let schema_json: Value = serde_json::from_str(&schema_content)?;

    // Compile schema
    let compiled = JSONSchema::compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))?;

    // Convert spec to JSON
    let spec_json = serde_json::to_value(spec)?;

    // Validate
    if let Err(errors) = compiled.validate(&spec_json) {
        let error_messages: Vec<String> = errors.map(|e| format!("{}", e)).collect();
        bail!("Schema validation failed:\n{}", error_messages.join("\n"));
    }

    Ok(())
}

/// Validate manifold against JSON schema (v2)
fn validate_against_schema_v2(manifold: &ManifoldV2) -> Result<()> {
    // Load schema
    let schema_path = crate::config::manifold_home()?.join("schemas/core-v2.json");
    let schema_content = std::fs::read_to_string(&schema_path)
        .map_err(|e| anyhow::anyhow!("Failed to read schema from {:?}: {}", schema_path, e))?;
    let schema_json: Value = serde_json::from_str(&schema_content)?;

    // Compile schema
    let compiled = JSONSchema::compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))?;

    // Convert manifold to JSON
    let manifold_json = serde_json::to_value(manifold)?;

    // Validate
    if let Err(errors) = compiled.validate(&manifold_json) {
        let error_messages: Vec<String> = errors.map(|e| format!("{}", e)).collect();
        bail!("Schema validation failed:\n{}", error_messages.join("\n"));
    }

    Ok(())
}
