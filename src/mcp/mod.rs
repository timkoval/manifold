//! MCP (Model Context Protocol) server for manifold
//!
//! Implements JSON-RPC 2.0 over stdio for LLM integration.
//! Tools exposed:
//! - create_spec: Create new spec
//! - apply_patch: Apply JSON patches to spec
//! - advance_workflow: Move spec between workflow stages
//! - query_manifold: Search/filter specs

use crate::config;
use crate::db::Database;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

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
                    "description": "Create a new specification in manifold. Returns the generated spec_id.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": {
                                "type": "string",
                                "description": "Project name (kebab-case, e.g., 'mobile-app', 'auth-service')"
                            },
                            "boundary": {
                                "type": "string",
                                "enum": ["personal", "work", "company"],
                                "description": "Isolation boundary: personal (private), work (team), company (org-wide)"
                            },
                            "name": {
                                "type": "string",
                                "description": "Human-readable spec name (e.g., 'User Authentication Service')"
                            }
                        },
                        "required": ["project", "boundary", "name"]
                    }
                },
                {
                    "name": "apply_patch",
                    "description": concat!(
                        "Apply JSON Patch (RFC 6902) operations to a spec. ",
                        "IMPORTANT: Only these paths are valid - unknown fields are silently dropped!\n\n",
                        "SPEC SCHEMA:\n",
                        "- /name (string): Spec name\n",
                        "- /requirements (array): List of requirements\n",
                        "- /tasks (array): List of tasks\n",
                        "- /decisions (array): List of design decisions\n\n",
                        "REQUIREMENT SCHEMA (for /requirements/- or /requirements/N):\n",
                        "{\n",
                        "  \"id\": \"REQ-001\",           // Required: unique ID\n",
                        "  \"capability\": \"auth\",       // Required: capability area\n",
                        "  \"title\": \"User Login\",      // Required: short title\n",
                        "  \"shall\": \"The system SHALL allow users to authenticate\",  // Required: SHALL statement\n",
                        "  \"rationale\": \"...\",         // Optional: why this requirement\n",
                        "  \"priority\": \"must\",         // Required: must|should|could|wont\n",
                        "  \"tags\": [\"security\"],       // Optional: array of tags\n",
                        "  \"scenarios\": []              // Optional: GIVEN/WHEN/THEN scenarios\n",
                        "}\n\n",
                        "SCENARIO SCHEMA (for /requirements/N/scenarios/-):\n",
                        "{\n",
                        "  \"id\": \"SCN-001\",\n",
                        "  \"name\": \"Valid login\",\n",
                        "  \"given\": [\"user exists\", \"password is correct\"],\n",
                        "  \"when\": \"user submits login form\",\n",
                        "  \"then\": [\"user is authenticated\", \"session is created\"],\n",
                        "  \"edge_cases\": []             // Optional\n",
                        "}\n\n",
                        "TASK SCHEMA (for /tasks/- or /tasks/N):\n",
                        "{\n",
                        "  \"id\": \"TASK-001\",           // Required: unique ID\n",
                        "  \"requirement_ids\": [\"REQ-001\"],  // Required: linked requirements\n",
                        "  \"title\": \"Implement login API\",   // Required\n",
                        "  \"description\": \"...\",       // Required: detailed description\n",
                        "  \"status\": \"pending\",        // Required: pending|in_progress|completed|blocked\n",
                        "  \"assignee\": \"@user\",        // Optional\n",
                        "  \"acceptance\": []             // Optional: acceptance criteria\n",
                        "}\n\n",
                        "DECISION SCHEMA (for /decisions/- or /decisions/N):\n",
                        "{\n",
                        "  \"id\": \"DEC-001\",            // Required: unique ID\n",
                        "  \"title\": \"Use JWT tokens\",  // Required\n",
                        "  \"context\": \"Need stateless auth\",  // Required: context/problem\n",
                        "  \"decision\": \"Use JWT with RS256\",  // Required: what was decided\n",
                        "  \"rationale\": \"...\",         // Required: why this decision\n",
                        "  \"alternatives_rejected\": [], // Optional: other options considered\n",
                        "  \"date\": \"2024-01-15\"        // Required: ISO date\n",
                        "}\n\n",
                        "EXAMPLES:\n",
                        "Add requirement: {\"op\":\"add\",\"path\":\"/requirements/-\",\"value\":{...}}\n",
                        "Update requirement title: {\"op\":\"replace\",\"path\":\"/requirements/0/title\",\"value\":\"New Title\"}\n",
                        "Add task: {\"op\":\"add\",\"path\":\"/tasks/-\",\"value\":{...}}\n",
                        "Remove decision: {\"op\":\"remove\",\"path\":\"/decisions/0\"}"
                    ),
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "spec_id": {
                                "type": "string",
                                "description": "Spec ID to patch (e.g., 'keen-grid-mobile')"
                            },
                            "patch": {
                                "type": "array",
                                "description": "Array of JSON Patch operations. Each operation: {\"op\": \"add|replace|remove\", \"path\": \"/requirements/-\", \"value\": {...}}",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "op": {"type": "string", "enum": ["add", "replace", "remove", "copy", "move", "test"]},
                                        "path": {"type": "string"},
                                        "value": {}
                                    },
                                    "required": ["op", "path"]
                                }
                            },
                            "summary": {
                                "type": "string",
                                "description": "Brief summary of changes (e.g., 'Added authentication requirements')"
                            }
                        },
                        "required": ["spec_id", "patch", "summary"]
                    }
                },
                {
                    "name": "advance_workflow",
                    "description": concat!(
                        "Move a spec to the next workflow stage. ",
                        "Stages must progress in order: requirements -> design -> tasks -> approval -> implemented. ",
                        "Each stage has validation rules that must pass before advancing."
                    ),
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
                    "description": "Search and filter specs. Returns list of specs with id, project, name, boundary, stage, and updated_at.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "boundary": {
                                "type": "string",
                                "enum": ["personal", "work", "company"],
                                "description": "Filter by boundary (optional)"
                            },
                            "stage": {
                                "type": "string",
                                "enum": ["requirements", "design", "tasks", "approval", "implemented"],
                                "description": "Filter by workflow stage (optional)"
                            },
                            "project": {
                                "type": "string",
                                "description": "Filter by project name - partial match (optional)"
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
