//! Conflict detection and resolution

use super::{Conflict, ConflictStatus, ResolutionStrategy};
use crate::models::SpecData;
use anyhow::{anyhow, Result};
use serde_json::Value;

/// Conflict resolver for merging specs
pub struct ConflictResolver;

impl ConflictResolver {
    /// Detect conflicts between local and remote specs
    pub fn detect_conflicts(
        local: &SpecData,
        remote: &SpecData,
        base: Option<&SpecData>,
    ) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();
        let now = chrono::Utc::now().timestamp();

        // Serialize to JSON for comparison
        let local_json = serde_json::to_value(local)?;
        let remote_json = serde_json::to_value(remote)?;
        let base_json = base.map(|b| serde_json::to_value(b)).transpose()?;

        // Check for conflicts in key fields
        conflicts.extend(Self::check_field_conflict(
            &local.spec_id,
            "name",
            &local_json["name"],
            &remote_json["name"],
            base_json.as_ref().and_then(|b| b.get("name")),
            now,
        )?);

        conflicts.extend(Self::check_field_conflict(
            &local.spec_id,
            "stage",
            &local_json["stage"],
            &remote_json["stage"],
            base_json.as_ref().and_then(|b| b.get("stage")),
            now,
        )?);

        // Check requirements conflicts
        conflicts.extend(Self::check_array_conflicts(
            &local.spec_id,
            "requirements",
            local_json.get("requirements"),
            remote_json.get("requirements"),
            base_json.as_ref().and_then(|b| b.get("requirements")),
            now,
        )?);

        // Check tasks conflicts
        conflicts.extend(Self::check_array_conflicts(
            &local.spec_id,
            "tasks",
            local_json.get("tasks"),
            remote_json.get("tasks"),
            base_json.as_ref().and_then(|b| b.get("tasks")),
            now,
        )?);

        // Check decisions conflicts
        conflicts.extend(Self::check_array_conflicts(
            &local.spec_id,
            "decisions",
            local_json.get("decisions"),
            remote_json.get("decisions"),
            base_json.as_ref().and_then(|b| b.get("decisions")),
            now,
        )?);

        Ok(conflicts)
    }

    /// Check if a single field has conflicts
    fn check_field_conflict(
        spec_id: &str,
        field_path: &str,
        local_value: &Value,
        remote_value: &Value,
        base_value: Option<&Value>,
        timestamp: i64,
    ) -> Result<Option<Conflict>> {
        // No conflict if values are the same
        if local_value == remote_value {
            return Ok(None);
        }

        // If we have a base, check if both sides changed
        if let Some(base) = base_value {
            let local_changed = local_value != base;
            let remote_changed = remote_value != base;

            // Conflict only if both changed
            if local_changed && remote_changed {
                return Ok(Some(Conflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    spec_id: spec_id.to_string(),
                    field_path: field_path.to_string(),
                    local_value: local_value.clone(),
                    remote_value: remote_value.clone(),
                    base_value: Some(base.clone()),
                    detected_at: timestamp,
                    status: ConflictStatus::Unresolved,
                }));
            }
        } else {
            // No base - assume it's a conflict
            return Ok(Some(Conflict {
                id: uuid::Uuid::new_v4().to_string(),
                spec_id: spec_id.to_string(),
                field_path: field_path.to_string(),
                local_value: local_value.clone(),
                remote_value: remote_value.clone(),
                base_value: None,
                detected_at: timestamp,
                status: ConflictStatus::Unresolved,
            }));
        }

        Ok(None)
    }

    /// Check conflicts in array fields (requirements, tasks, decisions)
    fn check_array_conflicts(
        spec_id: &str,
        field_name: &str,
        local_array: Option<&Value>,
        remote_array: Option<&Value>,
        base_array: Option<&Value>,
        timestamp: i64,
    ) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        let local_arr = local_array.and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let remote_arr = remote_array.and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let base_arr = base_array.and_then(|v| v.as_array()).cloned().unwrap_or_default();

        // Build maps by ID for easier comparison
        let local_map: std::collections::HashMap<String, &Value> = local_arr
            .iter()
            .filter_map(|item| {
                item.get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| (id.to_string(), item))
            })
            .collect();

        let remote_map: std::collections::HashMap<String, &Value> = remote_arr
            .iter()
            .filter_map(|item| {
                item.get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| (id.to_string(), item))
            })
            .collect();

        let base_map: std::collections::HashMap<String, &Value> = base_arr
            .iter()
            .filter_map(|item| {
                item.get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| (id.to_string(), item))
            })
            .collect();

        // Check each item in local
        for (id, local_item) in &local_map {
            if let Some(remote_item) = remote_map.get(id) {
                if local_item != remote_item {
                    let base_item = base_map.get(id);
                    
                    // Check if both changed
                    let local_changed = base_item.map_or(true, |base| local_item != base);
                    let remote_changed = base_item.map_or(true, |base| remote_item != base);

                    if local_changed && remote_changed {
                        conflicts.push(Conflict {
                            id: uuid::Uuid::new_v4().to_string(),
                            spec_id: spec_id.to_string(),
                            field_path: format!("{}/{}", field_name, id),
                            local_value: (*local_item).clone(),
                            remote_value: (*remote_item).clone(),
                            base_value: base_item.map(|v| (*v).clone()),
                            detected_at: timestamp,
                            status: ConflictStatus::Unresolved,
                        });
                    }
                }
            }
        }

        // Check for items added in remote but not in local
        for (id, remote_item) in &remote_map {
            if !local_map.contains_key(id) && base_map.contains_key(id) {
                // Item was deleted locally but modified remotely
                conflicts.push(Conflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    spec_id: spec_id.to_string(),
                    field_path: format!("{}/{}", field_name, id),
                    local_value: Value::Null,
                    remote_value: (*remote_item).clone(),
                    base_value: base_map.get(id).map(|v| (*v).clone()),
                    detected_at: timestamp,
                    status: ConflictStatus::Unresolved,
                });
            }
        }

        Ok(conflicts)
    }

    /// Resolve conflict with given strategy
    pub fn resolve_conflict(
        conflict: &Conflict,
        strategy: ResolutionStrategy,
        manual_value: Option<Value>,
    ) -> Result<(Value, ConflictStatus)> {
        match strategy {
            ResolutionStrategy::Ours => {
                Ok((conflict.local_value.clone(), ConflictStatus::ResolvedLocal))
            }
            ResolutionStrategy::Theirs => {
                Ok((conflict.remote_value.clone(), ConflictStatus::ResolvedRemote))
            }
            ResolutionStrategy::Manual => {
                if let Some(value) = manual_value {
                    Ok((value, ConflictStatus::ResolvedManual))
                } else {
                    Err(anyhow!("Manual resolution requires a value"))
                }
            }
            ResolutionStrategy::Merge => {
                // Attempt automatic merge for compatible changes
                Self::auto_merge(conflict)
            }
        }
    }

    /// Attempt automatic merge
    fn auto_merge(conflict: &Conflict) -> Result<(Value, ConflictStatus)> {
        // For arrays, try to merge non-conflicting items
        if let (Some(local_arr), Some(remote_arr)) = (
            conflict.local_value.as_array(),
            conflict.remote_value.as_array(),
        ) {
            let mut merged = local_arr.clone();
            
            // Add items from remote that aren't in local
            for remote_item in remote_arr {
                if let Some(remote_id) = remote_item.get("id") {
                    let exists = merged.iter().any(|item| item.get("id") == Some(remote_id));
                    if !exists {
                        merged.push(remote_item.clone());
                    }
                }
            }

            return Ok((Value::Array(merged), ConflictStatus::ResolvedManual));
        }

        // For simple values, can't auto-merge
        Err(anyhow!("Cannot auto-merge this conflict type"))
    }

    /// Apply resolved conflicts to spec
    pub fn apply_resolutions(
        spec: &mut SpecData,
        resolutions: &[(String, Value)],
    ) -> Result<()> {
        let mut spec_json = serde_json::to_value(&spec)?;

        for (field_path, value) in resolutions {
            // Parse field path (e.g., "name" or "requirements/req-001")
            let parts: Vec<&str> = field_path.split('/').collect();
            
            if parts.len() == 1 {
                // Top-level field
                spec_json[parts[0]] = value.clone();
            } else if parts.len() == 2 {
                // Array item by ID
                if let Some(array) = spec_json[parts[0]].as_array_mut() {
                    let id = parts[1];
                    if let Some(item) = array.iter_mut().find(|item| {
                        item.get("id").and_then(|v| v.as_str()) == Some(id)
                    }) {
                        *item = value.clone();
                    }
                }
            }
        }

        // Deserialize back to SpecData
        *spec = serde_json::from_value(spec_json)?;
        Ok(())
    }

    /// Get conflict summary for display
    pub fn format_conflict(conflict: &Conflict) -> String {
        format!(
            "Conflict in '{}'\n  Local:  {}\n  Remote: {}",
            conflict.field_path,
            Self::format_value(&conflict.local_value),
            Self::format_value(&conflict.remote_value)
        )
    }

    fn format_value(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Null => "(deleted)".to_string(),
            Value::Object(_) => "(modified object)".to_string(),
            Value::Array(arr) => format!("(array with {} items)", arr.len()),
            _ => value.to_string(),
        }
    }
}
