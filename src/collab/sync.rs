//! Git-based sync implementation

use super::{SyncConfig, SyncMetadata, SyncStatus};
use crate::models::SpecData;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Sync manager for git-based collaboration
pub struct SyncManager {
    config: SyncConfig,
}

impl SyncManager {
    pub fn new(config: SyncConfig) -> Self {
        Self { config }
    }

    /// Initialize a git repository for syncing
    pub fn init(&self) -> Result<()> {
        if !self.config.repo_path.exists() {
            fs::create_dir_all(&self.config.repo_path)
                .context("Failed to create sync directory")?;
        }

        // Initialize git repo
        let output = Command::new("git")
            .args(&["init"])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to initialize git repository")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Git init failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Configure git user
        Command::new("git")
            .args(&["config", "user.name", &self.config.commit_author])
            .current_dir(&self.config.repo_path)
            .output()?;

        Command::new("git")
            .args(&["config", "user.email", &self.config.commit_email])
            .current_dir(&self.config.repo_path)
            .output()?;

        println!("✓ Initialized sync repository at {:?}", self.config.repo_path);
        Ok(())
    }

    /// Export spec to git repository as JSON file
    pub fn export_spec(&self, spec: &SpecData) -> Result<PathBuf> {
        let spec_file = self.config.repo_path.join(format!("{}.json", spec.spec_id));
        
        let json = serde_json::to_string_pretty(spec)
            .context("Failed to serialize spec")?;
        
        fs::write(&spec_file, json)
            .context("Failed to write spec file")?;

        Ok(spec_file)
    }

    /// Import spec from git repository
    pub fn import_spec(&self, spec_id: &str) -> Result<SpecData> {
        let spec_file = self.config.repo_path.join(format!("{}.json", spec_id));
        
        if !spec_file.exists() {
            return Err(anyhow!("Spec file not found: {}", spec_id));
        }

        let json = fs::read_to_string(&spec_file)
            .context("Failed to read spec file")?;
        
        let spec: SpecData = serde_json::from_str(&json)
            .context("Failed to parse spec JSON")?;

        Ok(spec)
    }

    /// Commit changes to git
    pub fn commit(&self, message: &str, files: &[PathBuf]) -> Result<String> {
        // Stage files
        for file in files {
            let output = Command::new("git")
                .args(&["add", file.to_str().unwrap()])
                .current_dir(&self.config.repo_path)
                .output()
                .context("Failed to stage file")?;

            if !output.status.success() {
                return Err(anyhow!(
                    "Git add failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        // Commit
        let output = Command::new("git")
            .args(&["commit", "-m", message])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to commit changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Allow "nothing to commit" as success
            if stderr.contains("nothing to commit") {
                return Ok("no-changes".to_string());
            }
            return Err(anyhow!("Git commit failed: {}", stderr));
        }

        // Get commit hash
        let output = Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to get commit hash")?;

        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(hash)
    }

    /// Push changes to remote
    pub fn push(&self, remote: &str, branch: &str) -> Result<()> {
        let output = Command::new("git")
            .args(&["push", remote, branch])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to push to remote")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Git push failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        println!("✓ Pushed changes to {}/{}", remote, branch);
        Ok(())
    }

    /// Pull changes from remote
    pub fn pull(&self, remote: &str, branch: &str) -> Result<()> {
        let output = Command::new("git")
            .args(&["pull", remote, branch])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to pull from remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Check for conflicts
            if stderr.contains("CONFLICT") {
                return Err(anyhow!("Merge conflicts detected. Run 'manifold conflicts list' to see conflicts."));
            }
            return Err(anyhow!("Git pull failed: {}", stderr));
        }

        println!("✓ Pulled changes from {}/{}", remote, branch);
        Ok(())
    }

    /// Get sync status
    pub fn status(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(&["status", "--porcelain"])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to get git status")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Git status failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let status_text = String::from_utf8_lossy(&output.stdout);
        let modified_files: Vec<String> = status_text
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();

        Ok(modified_files)
    }

    /// Check if spec has local modifications
    pub fn is_modified(&self, spec_id: &str) -> Result<bool> {
        let spec_file = format!("{}.json", spec_id);
        let output = Command::new("git")
            .args(&["status", "--porcelain", &spec_file])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to check modification status")?;

        Ok(!output.stdout.is_empty())
    }

    /// Get file hash (for detecting changes)
    pub fn get_file_hash(&self, spec_id: &str) -> Result<String> {
        let spec_file = format!("{}.json", spec_id);
        let output = Command::new("git")
            .args(&["hash-object", &spec_file])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to get file hash")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Git hash-object failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// List all spec files in repository
    pub fn list_specs(&self) -> Result<Vec<String>> {
        let mut specs = Vec::new();
        
        if !self.config.repo_path.exists() {
            return Ok(specs);
        }

        for entry in fs::read_dir(&self.config.repo_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(spec_id) = stem.to_str() {
                        specs.push(spec_id.to_string());
                    }
                }
            }
        }

        Ok(specs)
    }

    /// Add remote repository
    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        let output = Command::new("git")
            .args(&["remote", "add", name, url])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to add remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                // Update existing remote
                Command::new("git")
                    .args(&["remote", "set-url", name, url])
                    .current_dir(&self.config.repo_path)
                    .output()?;
                println!("✓ Updated remote '{}' to {}", name, url);
            } else {
                return Err(anyhow!("Git remote add failed: {}", stderr));
            }
        } else {
            println!("✓ Added remote '{}' -> {}", name, url);
        }

        Ok(())
    }

    /// Get diff between local and remote
    pub fn diff(&self, spec_id: &str, remote: &str, branch: &str) -> Result<String> {
        let spec_file = format!("{}.json", spec_id);
        let output = Command::new("git")
            .args(&["diff", &format!("{}/{}", remote, branch), "--", &spec_file])
            .current_dir(&self.config.repo_path)
            .output()
            .context("Failed to get diff")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
