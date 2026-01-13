//! Registry commands for community manifolds

use crate::config::ManifoldPaths;
use crate::db::Database;

use crate::registry::{RegistryClient, RegistryConfig};
use anyhow::{bail, Context, Result};

/// Handle registry operations
pub fn registry_command(
    manifold_id: &str,
    username: &str,
    public_url: Option<&str>,
    description: Option<&str>,
    github_token: &str,
    branch: Option<&str>,
) -> Result<()> {
    let paths = ManifoldPaths::new()?;
    crate::commands::ensure_initialized(&paths)?;

    let db = Database::open(&paths)?;

    // Get manifold
    let manifold = db
        .get_manifold(manifold_id)?
        .context(format!("Manifold '{}' not found", manifold_id))?;

    // Validate that manifold has public boundaries
    let public_boundaries: Vec<_> = manifold
        .boundaries
        .iter()
        .filter(|(_, config)| {
            matches!(config.visibility, crate::models::BoundaryVisibility::Public)
        })
        .map(|(name, _)| name.clone())
        .collect();

    if public_boundaries.is_empty() {
        bail!("No public boundaries found in manifold. At least one boundary must have 'public' visibility to submit to registry.");
    }

    // Generate registry entry
    let public_url = public_url
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://github.com/{}/manifold/bundle.json", username));

    let description = description.map(|s| s.to_string()).unwrap_or_else(|| {
        format!(
            "Manifold with {} public boundaries",
            public_boundaries.len()
        )
    });

    let entry = RegistryClient::generate_entry_from_manifold(
        &manifold,
        username,
        &public_url,
        &description,
    )?;

    // Create registry client
    let registry_config = RegistryConfig::default();
    let client = RegistryClient::new(Some(github_token.to_string()))?;

    // Create pull request
    let branch_name = branch
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("manifold-{}", entry.manifold_id));

    println!("Submitting manifold '{}' to registry...", entry.manifold_id);
    println!("  Registry: {}", registry_config.registry_repo);
    println!("  Public boundaries: {:?}", public_boundaries);
    println!("  Branch: {}", branch_name);
    println!();

    let pr = client.submit_manifold(&entry, &branch_name)?;

    println!("✓ Pull request created!");
    println!("  URL: {}", pr.html_url);
    println!("  PR #{}", pr.number);

    Ok(())
}

/// List all manifolds in registry
pub fn registry_list(query: Option<&str>, json: bool, github_token: Option<&str>) -> Result<()> {
    let registry_config = RegistryConfig::default();
    let client = RegistryClient::new(github_token.map(|s| s.to_string()))?;

    let entries = client.fetch_entries()?;

    let filtered = if let Some(q) = query {
        entries
            .iter()
            .filter(|e| {
                e.description.to_lowercase().contains(&q.to_lowercase())
                    || e.manifold_id.to_lowercase().contains(&q.to_lowercase())
                    || e.user.to_lowercase().contains(&q.to_lowercase())
            })
            .cloned()
            .collect()
    } else {
        entries
    };

    if json {
        let json_output = serde_json::to_string_pretty(&filtered)?;
        println!("{}", json_output);
    } else {
        println!("Registry entries: {}", registry_config.registry_repo);
        println!("{}", "=".repeat(80));
        println!("{:<30} {:<20} {:<25}", "MANIFOLD ID", "USER", "DESCRIPTION");
        println!("{}", "-".repeat(80));

        for entry in &filtered {
            println!(
                "{:<30} {:<20} {:<25}",
                crate::commands::truncate(&entry.manifold_id, 28),
                crate::commands::truncate(&entry.user, 18),
                crate::commands::truncate(&entry.description, 23)
            );
        }

        if query.is_some() {
            println!();
            println!("Found {} matching entries", filtered.len());
        } else {
            println!();
            println!("Total entries: {}", filtered.len());
        }
    }

    Ok(())
}

/// Search manifolds in registry
pub fn registry_search(
    query: &str,
    limit: usize,
    github_token: Option<&str>,
    json: bool,
) -> Result<()> {
    let _registry_config = RegistryConfig::default();
    let client = RegistryClient::new(github_token.map(|s| s.to_string()))?;

    let mut results = client.search_entries(query)?;

    if results.len() > limit {
        results.truncate(limit);
    }

    if json {
        let json_output = serde_json::to_string_pretty(&results)?;
        println!("{}", json_output);
    } else {
        println!("Search results for: \"{}\"", query);
        println!("{}", "=".repeat(80));
        println!(
            "{:<25} {:<20} {:<25} {:<30}",
            "MANIFOLD ID", "USER", "UPDATED", "PUBLIC URL"
        );
        println!("{}", "-".repeat(80));

        for entry in &results {
            println!(
                "{:<25} {:<20} {:<25} {}",
                crate::commands::truncate(&entry.manifold_id, 23),
                crate::commands::truncate(&entry.user, 18),
                crate::commands::truncate(&entry.last_updated, 23),
                &entry.public_url
            );
        }

        println!();
        println!(
            "Found {} results (showing {})",
            results.len(),
            limit.min(results.len())
        );
    }

    Ok(())
}
