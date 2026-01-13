//! Agent runtime and MCP bridge for Manifold

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::config::ManifoldPaths;
use crate::db::Database;

/// A running agent handle
pub struct AgentHandle {
    pub id: String,
    pub thread: Option<JoinHandle<()>>,
}

/// Agent Manager to start/stop agents
#[derive(Clone, Default)]
pub struct AgentManager {
    inner: Arc<Mutex<HashMap<String, AgentHandle>>>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start a simple background agent that runs a closure periodically
    /// `id` is the agent identifier
    /// `interval_secs` - how often to run
    /// `task` - closure to run; it will receive a Database instance for safe operations
    pub fn start_agent<F>(&self, id: &str, interval_secs: u64, task: F) -> Result<()>
    where
        F: Fn(&Database) -> Result<()> + Send + 'static + Clone,
    {
        let id = id.to_string();
        let id_for_thread = id.clone();
        let inner = self.inner.clone();
        let paths = ManifoldPaths::new().context("Failed to load manifold paths")?;

        let thread_id = id_for_thread.clone();
        let handle = thread::spawn(move || {
            // Open DB inside thread
            let db = match Database::open(&paths) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Agent {}: failed to open database: {}", thread_id, e);
                    return;
                }
            };

            loop {
                if let Err(e) = task(&db) {
                    eprintln!("Agent {}: task error: {}", thread_id, e);
                }
                thread::sleep(Duration::from_secs(interval_secs));
                // Check if agent should stop
                let guard = inner.lock().unwrap();
                if !guard.contains_key(&id) {
                    break;
                }
            }
        });

        let mut guard = self.inner.lock().unwrap();
        guard.insert(
            id_for_thread.clone(),
            AgentHandle {
                id: id_for_thread.clone(),
                thread: Some(handle),
            },
        );

        Ok(())
    }

    /// Stop an agent by id
    pub fn stop_agent(&self, id: &str) -> Result<()> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(mut handle) = guard.remove(id) {
            // Dropping the entry will signal the thread to exit on next check
            // Optionally join
            if let Some(t) = handle.thread.take() {
                let _ = t.join();
            }
        }
        Ok(())
    }

    /// List agent ids
    pub fn list_agents(&self) -> Vec<String> {
        let guard = self.inner.lock().unwrap();
        guard.keys().cloned().collect()
    }
}

/// MCP Bridge: a minimal adapter that exposes safe helper functions agents can call.
pub struct McpBridge;

impl McpBridge {
    /// Simple helper to run a named tool with arguments by calling mcp::tools directly.
    /// This avoids going through JSON-RPC and keeps calls in-process.
    pub async fn call_tool(name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        // Initialize DB
        let paths = ManifoldPaths::new()?;
        let mut db = Database::open(&paths)?;

        match name {
            "create_spec" => crate::mcp::tools::create_spec(&mut db, args).await,
            "apply_patch" => crate::mcp::tools::apply_patch(&mut db, args).await,
            "advance_workflow" => crate::mcp::tools::advance_workflow(&mut db, args).await,
            "query_manifold" => crate::mcp::tools::query_manifold(&db, args).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }
}
