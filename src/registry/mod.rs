//! GitHub-based registry for community manifolds

use anyhow::{Context, Result};
// chrono::Utc is used via chrono::Utc::now() in generate_entry_from_manifold
use serde::{Deserialize, Serialize};

/// Registry entry for a manifold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// GitHub username or identifier
    pub user: String,

    /// Unique manifold ID
    pub manifold_id: String,

    /// Public URL to download the manifold bundle
    pub public_url: String,

    /// Description of the manifold
    pub description: String,

    /// Boundaries that are shared publicly
    pub boundaries_shared: Vec<String>,

    /// Last updated timestamp
    pub last_updated: String,
}

/// Registry entries file (entries.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntries {
    pub entries: Vec<RegistryEntry>,
}

/// GitHub PR response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestResponse {
    pub html_url: String,
    pub number: u64,
    pub state: String,
}

/// Registry configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// GitHub repository for the registry (e.g., manifold-community/registry)
    pub registry_repo: String,

    /// Base URL for GitHub API
    pub api_base: String,

    /// Entries file path in the registry
    pub entries_file: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            registry_repo: "manifold-community/registry".to_string(),
            api_base: "https://api.github.com".to_string(),
            entries_file: "registry/entries.json".to_string(),
        }
    }
}

/// Registry client for GitHub operations
pub struct RegistryClient {
    config: RegistryConfig,
    client: reqwest::blocking::Client,
    github_token: Option<String>,
}

impl RegistryClient {
    /// Create a new registry client
    pub fn new(github_token: Option<String>) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("manifold/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            config: RegistryConfig::default(),
            client,
            github_token,
        })
    }

    /// Create a custom registry client
    pub fn with_config(github_token: Option<String>, config: RegistryConfig) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("manifold/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            config,
            client,
            github_token,
        })
    }

    /// Fetch registry entries from GitHub
    pub fn fetch_entries(&self) -> Result<Vec<RegistryEntry>> {
        let url = format!(
            "{}/repos/{}/contents/{}",
            self.config.api_base, self.config.registry_repo, self.config.entries_file
        );

        let mut request = self.client.get(&url);
        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().context("Failed to fetch registry entries")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Failed to fetch registry: {} - {}", status, body);
        }

        let json = response.text().context("Failed to read response body")?;

        // GitHub API returns content in a specific format with base64 encoding
        // Parse GitHub API response to get the content field
        let api_response: serde_json::Value =
            serde_json::from_str(&json).context("Failed to parse GitHub API response")?;

        let content = api_response["content"]
            .as_str()
            .context("Missing 'content' field in GitHub API response")?;

        // Decode content (GitHub uses standard base64)
        let decoded = decode_content(content)?;
        let decoded_str = String::from_utf8(decoded).context("Failed to decode UTF-8")?;

        let entries: RegistryEntries =
            serde_json::from_str(&decoded_str).context("Failed to parse registry entries")?;

        Ok(entries.entries)
    }

    /// Search registry entries locally
    pub fn search_entries(&self, query: &str) -> Result<Vec<RegistryEntry>> {
        let entries = self.fetch_entries()?;

        let query_lower = query.to_lowercase();
        let filtered: Vec<_> = entries
            .into_iter()
            .filter(|entry| {
                entry.description.to_lowercase().contains(&query_lower)
                    || entry.manifold_id.to_lowercase().contains(&query_lower)
                    || entry.user.to_lowercase().contains(&query_lower)
                    || entry
                        .boundaries_shared
                        .iter()
                        .any(|b| b.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(filtered)
    }

    /// Create a pull request to submit a manifold to the registry
    pub fn submit_manifold(
        &self,
        entry: &RegistryEntry,
        branch_name: &str,
    ) -> Result<PullRequestResponse> {
        // First, get current entries
        let mut entries = self.fetch_entries()?;

        // Check if manifold already exists
        if entries.iter().any(|e| e.manifold_id == entry.manifold_id) {
            anyhow::bail!(
                "Manifold '{}' already exists in registry. Update existing entry or use a different ID.",
                entry.manifold_id
            );
        }

        // Add new entry
        entries.push(entry.clone());

        // Sort by last_updated
        entries.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

        // Create new branch
        let branch_url = format!(
            "{}/repos/{}/git/refs/heads/{}",
            self.config.api_base, self.config.registry_repo, branch_name
        );

        let mut request = self.client.get(&branch_url);
        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Check if branch exists
        let response = request.send();

        let _branch_exists = match response {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        };

        // If we have a GitHub token, attempt to create a branch, update entries.json and open a PR.
        if let Some(token) = &self.github_token {
            // Helper: parse owner/repo
            let parts: Vec<&str> = self.config.registry_repo.split('/').collect();
            if parts.len() == 2 {
                let owner = parts[0];
                let repo = parts[1];

                // 1) Get repository to find default branch and default commit SHA
                let repo_url = format!("{}/repos/{}/{}", self.config.api_base, owner, repo);
                let mut repo_req = self.client.get(&repo_url);
                repo_req = repo_req.header("Authorization", format!("Bearer {}", token));

                if let Ok(repo_resp) = repo_req.send() {
                    if repo_resp.status().is_success() {
                        if let Ok(repo_json) = repo_resp.json::<serde_json::Value>() {
                            let default_branch =
                                repo_json["default_branch"].as_str().unwrap_or("main");

                            // 2) Get reference for the default branch
                            let ref_url = format!(
                                "{}/repos/{}/{}/git/refs/heads/{}",
                                self.config.api_base, owner, repo, default_branch
                            );

                            let mut ref_req = self.client.get(&ref_url);
                            ref_req = ref_req.header("Authorization", format!("Bearer {}", token));

                            if let Ok(ref_resp) = ref_req.send() {
                                if ref_resp.status().is_success() {
                                    if let Ok(ref_json) = ref_resp.json::<serde_json::Value>() {
                                        let base_sha =
                                            ref_json["object"]["sha"].as_str().unwrap_or_default();

                                        // 3) Create new branch (ref) from base_sha
                                        let create_ref_url = format!(
                                            "{}/repos/{}/{}/git/refs",
                                            self.config.api_base, owner, repo
                                        );

                                        let create_ref_body = serde_json::json!({
                                            "ref": format!("refs/heads/{}", branch_name),
                                            "sha": base_sha,
                                        });

                                        let mut create_ref_req = self.client.post(&create_ref_url);
                                        create_ref_req = create_ref_req
                                            .header("Authorization", format!("Bearer {}", token));

                                        let create_ref_res = create_ref_req
                                            .json(&create_ref_body)
                                            .send()
                                            .context("Failed to create branch ref")?;

                                        if create_ref_res.status().is_success() {
                                            // 4) Get current entries.json file to obtain its blob SHA
                                            let contents_url = format!(
                                                "{}/repos/{}/{}/contents/{}",
                                                self.config.api_base,
                                                owner,
                                                repo,
                                                self.config.entries_file
                                            );

                                            let mut contents_req = self.client.get(&contents_url);
                                            contents_req = contents_req.header(
                                                "Authorization",
                                                format!("Bearer {}", token),
                                            );

                                            let contents_res = contents_req
                                                .send()
                                                .context("Failed to fetch entries file")?;

                                            if contents_res.status().is_success() {
                                                let contents_json =
                                                    contents_res.json::<serde_json::Value>()?;

                                                let file_sha = contents_json["sha"].as_str();

                                                // 5) Create a new blob with updated content
                                                let updated_content = serde_json::to_string_pretty(
                                                    &RegistryEntries {
                                                        entries: entries.clone(),
                                                    },
                                                )?;
                                                let blob_url = format!(
                                                    "{}/repos/{}/{}/git/blobs",
                                                    self.config.api_base, owner, repo
                                                );

                                                // Encode using base64 Engine
                                                use base64::engine::general_purpose::STANDARD;
                                                use base64::Engine;
                                                let encoded_content =
                                                    STANDARD.encode(updated_content.as_bytes());

                                                let blob_body = serde_json::json!({
                                                    "content": encoded_content,
                                                    "encoding": "base64"
                                                });

                                                let mut blob_req = self.client.post(&blob_url);
                                                blob_req = blob_req.header(
                                                    "Authorization",
                                                    format!("Bearer {}", token),
                                                );

                                                let blob_res = blob_req
                                                    .json(&blob_body)
                                                    .send()
                                                    .context("Failed to create blob")?;
                                                if blob_res.status().is_success() {
                                                    let blob_json =
                                                        blob_res.json::<serde_json::Value>()?;
                                                    let blob_sha = blob_json["sha"]
                                                        .as_str()
                                                        .context("Missing blob sha")?;

                                                    // 6) Create new tree with the updated file
                                                    let tree_url = format!(
                                                        "{}/repos/{}/{}/git/trees",
                                                        self.config.api_base, owner, repo
                                                    );

                                                    let tree_body = serde_json::json!({
                                                        "base_tree": base_sha,
                                                        "tree": [
                                                            {
                                                                "path": self.config.entries_file,
                                                                "mode": "100644",
                                                                "type": "blob",
                                                                "sha": blob_sha
                                                            }
                                                        ]
                                                    });

                                                    let mut tree_req = self.client.post(&tree_url);
                                                    tree_req = tree_req.header(
                                                        "Authorization",
                                                        format!("Bearer {}", token),
                                                    );

                                                    let tree_res = tree_req
                                                        .json(&tree_body)
                                                        .send()
                                                        .context("Failed to create tree")?;
                                                    if tree_res.status().is_success() {
                                                        let tree_json =
                                                            tree_res.json::<serde_json::Value>()?;
                                                        let tree_sha = tree_json["sha"]
                                                            .as_str()
                                                            .context("Missing tree sha")?;

                                                        // 7) Create a commit
                                                        let commit_url = format!(
                                                            "{}/repos/{}/{}/git/commits",
                                                            self.config.api_base, owner, repo
                                                        );

                                                        let commit_body = serde_json::json!({
                                                            "message": format!("Add manifold {} to registry", entry.manifold_id),
                                                            "tree": tree_sha,
                                                            "parents": [base_sha]
                                                        });

                                                        let mut commit_req =
                                                            self.client.post(&commit_url);
                                                        commit_req = commit_req.header(
                                                            "Authorization",
                                                            format!("Bearer {}", token),
                                                        );

                                                        let commit_res = commit_req
                                                            .json(&commit_body)
                                                            .send()
                                                            .context("Failed to create commit")?;

                                                        if commit_res.status().is_success() {
                                                            let commit_json = commit_res
                                                                .json::<serde_json::Value>(
                                                            )?;
                                                            let commit_sha = commit_json["sha"]
                                                                .as_str()
                                                                .context("Missing commit sha")?;

                                                            // 8) Update branch ref to point to new commit
                                                            let update_ref_url = format!(
                                                                "{}/repos/{}/{}/git/refs/heads/{}",
                                                                self.config.api_base,
                                                                owner,
                                                                repo,
                                                                branch_name
                                                            );

                                                            let update_body = serde_json::json!({
                                                                "sha": commit_sha,
                                                                "force": false
                                                            });

                                                            let mut update_req =
                                                                self.client.patch(&update_ref_url);
                                                            update_req = update_req.header(
                                                                "Authorization",
                                                                format!("Bearer {}", token),
                                                            );

                                                            let update_res = update_req
                                                                .json(&update_body)
                                                                .send()
                                                                .context(
                                                                    "Failed to update branch ref",
                                                                )?;
                                                            if update_res.status().is_success() {
                                                                // 9) Create PR
                                                                let pr_url = format!(
                                                                    "{}/repos/{}/{}/pulls",
                                                                    self.config.api_base,
                                                                    owner,
                                                                    repo
                                                                );

                                                                let pr_body = serde_json::json!({
                                                                    "title": format!("Add manifold: {}", entry.manifold_id),
                                                                    "body": format!(
                                                                        "Submitted by @{}\n\nManifold ID: {}\nDescription: {}\nBoundaries: {:?}",
                                                                        entry.user, entry.manifold_id, entry.description, entry.boundaries_shared
                                                                    ),
                                                                    "head": branch_name,
                                                                    "base": default_branch,
                                                                    "maintainer_can_modify": true
                                                                });

                                                                let mut pr_req =
                                                                    self.client.post(&pr_url);
                                                                pr_req = pr_req.header(
                                                                    "Authorization",
                                                                    format!("Bearer {}", token),
                                                                );

                                                                let pr_res = pr_req
                                                                    .json(&pr_body)
                                                                    .send()
                                                                    .context(
                                                                        "Failed to create PR",
                                                                    )?;

                                                                if pr_res.status().is_success() {
                                                                    let pr_json = pr_res.json::<serde_json::Value>()?;
                                                                    let pr = PullRequestResponse {
                                                                        html_url: pr_json
                                                                            ["html_url"]
                                                                            .as_str()
                                                                            .unwrap_or_default()
                                                                            .to_string(),
                                                                        number: pr_json["number"]
                                                                            .as_u64()
                                                                            .unwrap_or_default(),
                                                                        state: pr_json["state"]
                                                                            .as_str()
                                                                            .unwrap_or_default()
                                                                            .to_string(),
                                                                    };

                                                                    return Ok(pr);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // If we reached here, something failed in the API flow; fall back to simulated PR creation
        }

        // Fallback: create PR via direct API call (may fail if branch doesn't actually contain the file)
        let pr_url = format!(
            "{}/repos/{}/pulls",
            self.config.api_base, self.config.registry_repo
        );

        let mut pr_request = self.client.post(&pr_url);
        if let Some(token) = &self.github_token {
            pr_request = pr_request.header("Authorization", format!("Bearer {}", token));
        }

        let pr_body = serde_json::json!({
            "title": format!("Add manifold: {}", entry.manifold_id),
            "body": format!(
                "Submitted by @{}\n\nManifold ID: {}\nDescription: {}\nBoundaries: {:?}",
                entry.user, entry.manifold_id, entry.description, entry.boundaries_shared
            ),
            "head": branch_name,
            "base": "main",
            "maintainer_can_modify": true
        });

        let pr_response = pr_request
            .json(&pr_body)
            .send()
            .context("Failed to create pull request")?;

        if !pr_response.status().is_success() {
            let status = pr_response.status();
            let body = pr_response.text().unwrap_or_default();
            anyhow::bail!("Failed to create PR: {} - {}", status, body);
        }

        let pr: PullRequestResponse = pr_response.json().context("Failed to parse PR response")?;

        Ok(pr)
    }

    /// Generate a registry entry from a manifold (v2)
    pub fn generate_entry_from_manifold(
        manifold: &crate::models::ManifoldV2,
        username: &str,
        public_url: &str,
        description: &str,
    ) -> Result<RegistryEntry> {
        // Extract boundaries that are "public" or shared
        let boundaries_shared: Vec<_> = manifold
            .boundaries
            .iter()
            .filter(|(_, config)| {
                matches!(config.visibility, crate::models::BoundaryVisibility::Public)
            })
            .map(|(name, _)| name.clone())
            .collect();

        if boundaries_shared.is_empty() {
            anyhow::bail!(
                "No public boundaries found in manifold. At least one boundary must have 'public' visibility to submit to registry."
            );
        }

        let last_updated = chrono::Utc::now().format("%Y-%m-%d").to_string();

        Ok(RegistryEntry {
            user: username.to_string(),
            manifold_id: manifold.manifold_id.clone(),
            public_url: public_url.to_string(),
            description: description.to_string(),
            boundaries_shared,
            last_updated,
        })
    }
}

/// Decode GitHub API `content` field which is base64 (may contain newlines)
fn decode_content(input: &str) -> Result<Vec<u8>> {
    let s = input.trim();
    // GitHub uses standard base64 with newlines allowed; safe to remove whitespace
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        STANDARD
            .decode(&cleaned)
            .context("Failed to base64-decode GitHub content")
    }
}
