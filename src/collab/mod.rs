//! Collaboration features for manifold
//!
//! Provides git-based sync, conflict resolution, and review workflows

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

pub mod sync;
pub mod conflicts;
pub mod reviews;

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub repo_path: PathBuf,
    pub remote_url: Option<String>,
    pub auto_commit: bool,
    pub commit_author: String,
    pub commit_email: String,
}

impl SyncConfig {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            remote_url: None,
            auto_commit: true,
            commit_author: "Manifold".to_string(),
            commit_email: "manifold@local".to_string(),
        }
    }
}

/// Sync metadata tracked per spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub spec_id: String,
    pub last_sync_timestamp: i64,
    pub last_sync_hash: String,
    pub remote_branch: Option<String>,
    pub sync_status: SyncStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Synced,
    Modified,
    Conflicted,
    Unsynced,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Synced => write!(f, "synced"),
            SyncStatus::Modified => write!(f, "modified"),
            SyncStatus::Conflicted => write!(f, "conflicted"),
            SyncStatus::Unsynced => write!(f, "unsynced"),
        }
    }
}

impl FromStr for SyncStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "synced" => Ok(SyncStatus::Synced),
            "modified" => Ok(SyncStatus::Modified),
            "conflicted" => Ok(SyncStatus::Conflicted),
            "unsynced" => Ok(SyncStatus::Unsynced),
            _ => Err(format!("Invalid sync status: {}", s)),
        }
    }
}

/// Conflict between local and remote versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub spec_id: String,
    pub field_path: String,
    pub local_value: serde_json::Value,
    pub remote_value: serde_json::Value,
    pub base_value: Option<serde_json::Value>,
    pub detected_at: i64,
    pub status: ConflictStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStatus {
    Unresolved,
    ResolvedLocal,
    ResolvedRemote,
    ResolvedManual,
}

impl std::fmt::Display for ConflictStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictStatus::Unresolved => write!(f, "unresolved"),
            ConflictStatus::ResolvedLocal => write!(f, "resolved_local"),
            ConflictStatus::ResolvedRemote => write!(f, "resolved_remote"),
            ConflictStatus::ResolvedManual => write!(f, "resolved_manual"),
        }
    }
}

impl FromStr for ConflictStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unresolved" => Ok(ConflictStatus::Unresolved),
            "resolved_local" => Ok(ConflictStatus::ResolvedLocal),
            "resolved_remote" => Ok(ConflictStatus::ResolvedRemote),
            "resolved_manual" => Ok(ConflictStatus::ResolvedManual),
            _ => Err(format!("Invalid conflict status: {}", s)),
        }
    }
}

/// Review request for spec approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub spec_id: String,
    pub requester: String,
    pub reviewer: String,
    pub status: ReviewStatus,
    pub comment: Option<String>,
    pub requested_at: i64,
    pub reviewed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl std::fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewStatus::Pending => write!(f, "pending"),
            ReviewStatus::Approved => write!(f, "approved"),
            ReviewStatus::Rejected => write!(f, "rejected"),
            ReviewStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl FromStr for ReviewStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(ReviewStatus::Pending),
            "approved" => Ok(ReviewStatus::Approved),
            "rejected" => Ok(ReviewStatus::Rejected),
            "cancelled" => Ok(ReviewStatus::Cancelled),
            _ => Err(format!("Invalid review status: {}", s)),
        }
    }
}

/// Resolution strategy for conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStrategy {
    Ours,      // Keep local changes
    Theirs,    // Accept remote changes
    Manual,    // User will resolve manually
    Merge,     // Attempt automatic merge
}
