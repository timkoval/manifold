//! MCP (Model Context Protocol) server for manifold
//! 
//! Implements JSON-RPC 2.0 over stdio for LLM integration.
//! Tools exposed:
//! - create_spec: Create new spec
//! - apply_patch: Apply JSON patches to spec
//! - advance_workflow: Move spec between workflow stages
//! - query_manifold: Search/filter specs

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use crate::db::Database;
use crate::config;

mod tools;

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP Server
pub struct McpServer {
    db: Database,
}

impl McpServer {
    pub fn new() -> Result<Self> {
        let paths = config::ManifoldPaths::new()?;
        let db = Database::open(&paths)?;
        Ok(Self { db })
    }

    /// Run the MCP server (stdio JSON-RPC 2.0)
    pub async fn run(&mut self) -> Result<()> {
        eprintln!("Manifold MCP server starting...");
        eprintln!("Protocol: JSON-RPC 2.0 over stdio");
        eprintln!("Available tools:");
        eprintln!("  - create_spec");
        eprintln!("  - apply_patch");
        eprintln!("  - advance_workflow");
        eprintln!("  - query_manifold");
        eprintln!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();
        
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let response = self.handle_request(&line).await;
            let response_json = serde_json::to_string(&response)?;
            writeln!(stdout, "{}", response_json)?;
            stdout.flush()?;
        }

        Ok(())
    }

    /// Handle a single JSON-RPC request
    async fn handle_request(&mut self, request_str: &str) -> JsonRpcResponse {
        // Parse request
        let request: JsonRpcRequest = match serde_json::from_str(request_str) {
            Ok(req) => req,
            Err(e) => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: "Invalid Request: jsonrpc must be '2.0'".to_string(),
                    data: None,
                }),
            };
        }

        // Handle MCP methods
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tool_call(request.params).await,
            _ => Err(anyhow::anyhow!("Method not found: {}", request.method)),
        };

        match result {
            Ok(result_value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(result_value),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: format!("Internal error: {}", e),
                    data: None,
                }),
            },
        }
    }

    /// Handle MCP initialize
    async fn handle_initialize(&self, _params: Option<Value>) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "manifold",
                "version": "0.1.0"
            },
            "capabilities": {
                "tools": {}
            }
        }))
    }

    /// Handle tools/list
    async fn handle_tools_list(&self) -> Result<Value> {
        Ok(json!({
            "tools": [
                {
                    "name": "create_spec",
                    "description": "Create a new specification in manifold",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": {
                                "type": "string",
                                "description": "Project name (kebab-case)"
                            },
                            "boundary": {
                                "type": "string",
                                "enum": ["personal", "work", "company"],
                                "description": "Boundary isolation level"
                            },
                            "name": {
                                "type": "string",
                                "description": "Human-readable spec name"
                            },
                            "description": {
                                "type": "string",
                                "description": "Spec description"
                            }
                        },
                        "required": ["project", "boundary", "name"]
                    }
                },
                {
                    "name": "apply_patch",
                    "description": "Apply a JSON patch to a spec",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "spec_id": {
                                "type": "string",
                                "description": "Spec ID to patch"
                            },
                            "patch": {
                                "type": "array",
                                "description": "JSON Patch operations (RFC 6902)"
                            },
                            "summary": {
                                "type": "string",
                                "description": "Summary of changes"
                            }
                        },
                        "required": ["spec_id", "patch", "summary"]
                    }
                },
                {
                    "name": "advance_workflow",
                    "description": "Move a spec to the next workflow stage",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "spec_id": {
                                "type": "string",
                                "description": "Spec ID to advance"
                            },
                            "target_stage": {
                                "type": "string",
                                "enum": ["requirements", "design", "tasks", "approval", "implemented"],
                                "description": "Target workflow stage"
                            }
                        },
                        "required": ["spec_id", "target_stage"]
                    }
                },
                {
                    "name": "query_manifold",
                    "description": "Search and filter specs in manifold",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "boundary": {
                                "type": "string",
                                "enum": ["personal", "work", "company"],
                                "description": "Filter by boundary"
                            },
                            "stage": {
                                "type": "string",
                                "enum": ["requirements", "design", "tasks", "approval", "implemented"],
                                "description": "Filter by workflow stage"
                            },
                            "project": {
                                "type": "string",
                                "description": "Filter by project name (partial match)"
                            }
                        }
                    }
                }
            ]
        }))
    }

    /// Handle tools/call
    async fn handle_tool_call(&mut self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("Missing params"))?;
        let tool_name = params["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
        let arguments = params["arguments"].clone();

        match tool_name {
            "create_spec" => tools::create_spec(&mut self.db, arguments).await,
            "apply_patch" => tools::apply_patch(&mut self.db, arguments).await,
            "advance_workflow" => tools::advance_workflow(&mut self.db, arguments).await,
            "query_manifold" => tools::query_manifold(&self.db, arguments).await,
            _ => bail!("Unknown tool: {}", tool_name),
        }
    }
}
