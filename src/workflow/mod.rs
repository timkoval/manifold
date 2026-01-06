//! Workflow engine for managing spec lifecycle transitions
//!
//! Implements state machine with validation rules:
//! - requirements -> design: Must have at least one requirement
//! - design -> tasks: Must have at least one decision
//! - tasks -> approval: Must have at least one task
//! - approval -> implemented: Manual approval only
//!
//! All transitions are logged to workflow_events table

use crate::models::{SpecData, WorkflowStage};
use std::time::{SystemTime, UNIX_EPOCH};

/// Workflow event types
/// Core types for workflow event tracking in database
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WorkflowEvent {
    /// Stage transition (from, to)
    Transition(WorkflowStage, WorkflowStage),
    /// Stage completion
    StageCompleted(WorkflowStage),
    /// Validation failed
    ValidationFailed(String),
    /// Manual approval
    Approved(String),
    /// Manual rejection
    Rejected(String),
}

impl WorkflowEvent {
    pub fn as_string(&self) -> String {
        match self {
            WorkflowEvent::Transition(from, to) => {
                format!("transition:{}:{}", from, to)
            }
            WorkflowEvent::StageCompleted(stage) => {
                format!("completed:{}", stage)
            }
            WorkflowEvent::ValidationFailed(msg) => {
                format!("validation_failed:{}", msg)
            }
            WorkflowEvent::Approved(approver) => {
                format!("approved:{}", approver)
            }
            WorkflowEvent::Rejected(reason) => {
                format!("rejected:{}", reason)
            }
        }
    }
}

/// Workflow validation error
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Invalid transition from {from} to {to}: {reason}")]
    InvalidTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Already at stage: {0}")]
    AlreadyAtStage(String),

    #[error("Cannot go backwards from {from} to {to}")]
    BackwardTransition { from: String, to: String },
}

/// Workflow engine for managing spec transitions
pub struct WorkflowEngine;

impl WorkflowEngine {
    /// Validate and execute a stage transition
    pub fn advance_stage(
        spec: &SpecData,
        target_stage: WorkflowStage,
    ) -> Result<WorkflowTransition, WorkflowError> {
        let current = &spec.stage;

        // Check if already at target stage
        if current == &target_stage {
            return Err(WorkflowError::AlreadyAtStage(target_stage.to_string()));
        }

        // Check if trying to go backwards
        if Self::stage_index(&target_stage) <= Self::stage_index(current) {
            return Err(WorkflowError::BackwardTransition {
                from: current.to_string(),
                to: target_stage.to_string(),
            });
        }

        // Validate the transition
        Self::validate_transition(spec, current, &target_stage)?;

        // Create transition event
        let event = WorkflowEvent::Transition(current.clone(), target_stage.clone());

        Ok(WorkflowTransition {
            from: current.clone(),
            to: target_stage,
            event,
        })
    }

    /// Validate that a spec can transition to the next stage
    fn validate_transition(
        spec: &SpecData,
        from: &WorkflowStage,
        to: &WorkflowStage,
    ) -> Result<(), WorkflowError> {
        match (from, to) {
            // requirements -> design: Must have at least one requirement
            (WorkflowStage::Requirements, WorkflowStage::Design) => {
                if spec.requirements.is_empty() {
                    return Err(WorkflowError::ValidationFailed(
                        "Cannot advance to design: no requirements defined".to_string(),
                    ));
                }

                // Check that at least one requirement has a SHALL statement
                let has_shall = spec.requirements.iter().any(|r| !r.shall.is_empty());
                if !has_shall {
                    return Err(WorkflowError::ValidationFailed(
                        "Cannot advance to design: no SHALL statements defined".to_string(),
                    ));
                }

                Ok(())
            }

            // design -> tasks: Must have at least one decision
            (WorkflowStage::Design, WorkflowStage::Tasks) => {
                if spec.decisions.is_empty() {
                    return Err(WorkflowError::ValidationFailed(
                        "Cannot advance to tasks: no design decisions documented".to_string(),
                    ));
                }
                Ok(())
            }

            // tasks -> approval: Must have at least one task
            (WorkflowStage::Tasks, WorkflowStage::Approval) => {
                if spec.tasks.is_empty() {
                    return Err(WorkflowError::ValidationFailed(
                        "Cannot advance to approval: no tasks defined".to_string(),
                    ));
                }

                // Check that all tasks have requirement traceability
                for task in &spec.tasks {
                    if task.requirement_ids.is_empty() {
                        return Err(WorkflowError::ValidationFailed(format!(
                            "Task {} has no requirement traceability",
                            task.id
                        )));
                    }
                }

                Ok(())
            }

            // approval -> implemented: No automatic validation (manual approval required)
            (WorkflowStage::Approval, WorkflowStage::Implemented) => Ok(()),

            // All other transitions
            _ => Err(WorkflowError::InvalidTransition {
                from: from.to_string(),
                to: to.to_string(),
                reason: "Must advance through stages sequentially".to_string(),
            }),
        }
    }

    /// Get numeric index for stage (for comparison)
    fn stage_index(stage: &WorkflowStage) -> usize {
        match stage {
            WorkflowStage::Requirements => 0,
            WorkflowStage::Design => 1,
            WorkflowStage::Tasks => 2,
            WorkflowStage::Approval => 3,
            WorkflowStage::Implemented => 4,
        }
    }

    /// Get the next stage in the workflow
    pub fn next_stage(current: &WorkflowStage) -> Option<WorkflowStage> {
        match current {
            WorkflowStage::Requirements => Some(WorkflowStage::Design),
            WorkflowStage::Design => Some(WorkflowStage::Tasks),
            WorkflowStage::Tasks => Some(WorkflowStage::Approval),
            WorkflowStage::Approval => Some(WorkflowStage::Implemented),
            WorkflowStage::Implemented => None,
        }
    }

    /// Check if a stage can be advanced
    pub fn can_advance(spec: &SpecData) -> Result<WorkflowStage, WorkflowError> {
        let current = &spec.stage;

        match Self::next_stage(current) {
            Some(next) => {
                Self::validate_transition(spec, current, &next)?;
                Ok(next)
            }
            None => Err(WorkflowError::ValidationFailed(
                "Already at final stage (implemented)".to_string(),
            )),
        }
    }
}

/// Result of a successful workflow transition
#[derive(Debug)]
pub struct WorkflowTransition {
    pub from: WorkflowStage,
    pub to: WorkflowStage,
    pub event: WorkflowEvent,
}

impl WorkflowTransition {
    /// Get the current timestamp
    /// Utility for creating workflow events
    #[allow(dead_code)]
    pub fn timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Boundary, Priority, Requirement};

    fn create_test_spec(stage: WorkflowStage) -> SpecData {
        let mut spec = SpecData::new(
            "test-spec".to_string(),
            "test-project".to_string(),
            "Test Spec".to_string(),
            Boundary::Personal,
        );
        spec.stage = stage;
        spec
    }

    #[test]
    fn test_cannot_advance_without_requirements() {
        let spec = create_test_spec(WorkflowStage::Requirements);
        let result = WorkflowEngine::advance_stage(&spec, WorkflowStage::Design);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no requirements"));
    }

    #[test]
    fn test_can_advance_with_requirements() {
        let mut spec = create_test_spec(WorkflowStage::Requirements);
        spec.requirements.push(Requirement {
            id: "req-1".to_string(),
            capability: "test".to_string(),
            title: "Test".to_string(),
            shall: "The system SHALL do something".to_string(),
            rationale: None,
            priority: Priority::Must,
            tags: vec![],
            scenarios: vec![],
        });

        let result = WorkflowEngine::advance_stage(&spec, WorkflowStage::Design);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cannot_advance_without_decisions() {
        let mut spec = create_test_spec(WorkflowStage::Design);
        spec.requirements.push(Requirement {
            id: "req-1".to_string(),
            capability: "test".to_string(),
            title: "Test".to_string(),
            shall: "SHALL do something".to_string(),
            rationale: None,
            priority: Priority::Must,
            tags: vec![],
            scenarios: vec![],
        });

        let result = WorkflowEngine::advance_stage(&spec, WorkflowStage::Tasks);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no design decisions"));
    }

    #[test]
    fn test_cannot_go_backwards() {
        let spec = create_test_spec(WorkflowStage::Design);
        let result = WorkflowEngine::advance_stage(&spec, WorkflowStage::Requirements);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("backwards"));
    }
}
