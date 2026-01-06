//! LLM editing loop for conversational spec manipulation
//!
//! Provides interactive session for editing specs with LLM assistance

use anyhow::{Context, Result};
use rustyline::{error::ReadlineError, DefaultEditor};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::ManifoldPaths;
use crate::db::Database;
use crate::models::SpecData;

/// LLM API configuration
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_url: std::env::var("OPENAI_API_BASE")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            api_key: std::env::var("OPENAI_API_KEY")
                .unwrap_or_else(|_| "sk-dummy-key-for-testing".to_string()),
            model: std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
        }
    }
}

/// LLM chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// LLM chat completion response
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// Interactive LLM editing session
pub struct LlmSession {
    spec_id: String,
    db: Database,
    llm_config: LlmConfig,
    conversation_history: Vec<ChatMessage>,
    client: reqwest::Client,
    llm_enabled: bool,
}

impl LlmSession {
    /// Create a new LLM editing session
    pub fn new(spec_id: String, paths: &ManifoldPaths) -> Result<Self> {
        let db = Database::open(paths)?;

        // Load config and use it for LLM settings
        let config = crate::config::load_config()?;
        let llm_config = LlmConfig {
            api_url: config.llm.endpoint.unwrap_or_else(|| {
                std::env::var("OPENAI_API_BASE")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string())
            }),
            api_key: std::env::var("OPENAI_API_KEY")
                .unwrap_or_else(|_| "sk-dummy-key-for-testing".to_string()),
            model: config.llm.model.unwrap_or_else(|| {
                std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4".to_string())
            }),
        };

        // Check if API key is set (allow dummy key for testing)
        let llm_enabled =
            !llm_config.api_key.is_empty() && llm_config.api_key != "sk-dummy-key-for-testing";

        Ok(Self {
            spec_id,
            db,
            llm_config,
            conversation_history: Vec::new(),
            client: reqwest::Client::new(),
            llm_enabled,
        })
    }

    /// Start the interactive editing loop
    pub async fn run(&mut self) -> Result<()> {
        // Load initial spec
        let spec = self.load_spec()?;

        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║  Manifold LLM Editing Session                                ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();
        println!("Editing spec: {}", self.spec_id);
        println!("Current stage: {}", spec.stage);
        println!("Requirements: {}", spec.requirements.len());
        println!("Tasks: {}", spec.tasks.len());
        println!("Decisions: {}", spec.decisions.len());
        println!();

        if !self.llm_enabled {
            println!("⚠️  LLM API not configured (OPENAI_API_KEY not set)");
            println!("   Running in command-only mode.");
            println!();
        }

        println!("Commands:");
        println!("  /status     - Show current spec status");
        println!("  /advance    - Advance workflow stage");
        println!("  /show       - Show full spec JSON");
        println!("  /exit       - Exit session");
        println!();

        if self.llm_enabled {
            println!("Type your message to chat with the AI about your spec...");
        } else {
            println!("Use commands above to navigate the spec.");
        }
        println!();

        // Initialize system prompt
        self.init_system_prompt(&spec);

        // Start REPL
        let mut rl = DefaultEditor::new()?;

        loop {
            let prompt = if self.llm_enabled {
                "You> "
            } else {
                "manifold> "
            };
            let readline = rl.readline(prompt);

            match readline {
                Ok(line) => {
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        continue;
                    }

                    rl.add_history_entry(trimmed)?;

                    // Handle commands
                    if trimmed.starts_with('/') {
                        match self.handle_command(trimmed).await {
                            Ok(should_exit) => {
                                if should_exit {
                                    break;
                                }
                            }
                            Err(e) => {
                                println!("✗ Error: {}", e);
                            }
                        }
                        continue;
                    }

                    // Send to LLM if enabled
                    if self.llm_enabled {
                        match self.chat(trimmed).await {
                            Ok(response) => {
                                println!("\nAI> {}\n", response);
                            }
                            Err(e) => {
                                println!("✗ LLM Error: {}", e);
                            }
                        }
                    } else {
                        println!("LLM not enabled. Use /exit to quit or set OPENAI_API_KEY.");
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Interrupted. Use /exit to quit.");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("EOF");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        println!("\nSession ended.");
        Ok(())
    }

    /// Initialize system prompt with spec context
    fn init_system_prompt(&mut self, spec: &SpecData) {
        let spec_json = serde_json::to_string_pretty(spec).unwrap_or_default();

        let system_prompt = format!(
            r#"You are an expert requirements engineer helping to edit a specification in the Manifold system.

Current spec (JSON format):
```json
{}
```

The spec follows this structure:
- requirements: Array of requirement objects with SHALL statements, scenarios (GIVEN/WHEN/THEN), priorities (must/should/could/wont)
- tasks: Array of task objects with requirement traceability
- decisions: Array of design decisions with rationale

Workflow stages: requirements → design → tasks → approval → implemented

Your role:
1. Help the user understand and improve their specification
2. Suggest new requirements using precise SHALL/SHALL NOT language
3. Recommend GIVEN/WHEN/THEN scenarios for requirements
4. Guide design decision documentation
5. Advise on workflow stage advancement readiness
6. Provide specific, actionable feedback

When the user asks to add content, provide them with the exact JSON structure they should add.
Be concise and practical. Focus on quality requirements engineering practices."#,
            spec_json
        );

        self.conversation_history.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        });
    }

    /// Send a message to the LLM and get response
    async fn chat(&mut self, user_message: &str) -> Result<String> {
        // Add user message to history
        self.conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        // Prepare API request
        let request_body = json!({
            "model": self.llm_config.model,
            "messages": self.conversation_history,
            "temperature": 0.7,
            "max_tokens": 1500,
        });

        // Call LLM API
        let response = self
            .client
            .post(format!("{}/chat/completions", self.llm_config.api_url))
            .header(
                "Authorization",
                format!("Bearer {}", self.llm_config.api_key),
            )
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error ({}): {}", status, error_text);
        }

        let completion: ChatCompletionResponse = response
            .json()
            .await
            .context("Failed to parse LLM response")?;

        let assistant_message = completion
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?
            .message
            .clone();

        // Add to history
        self.conversation_history.push(assistant_message.clone());

        Ok(assistant_message.content)
    }

    /// Handle slash commands
    async fn handle_command(&mut self, command: &str) -> Result<bool> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let cmd = parts.first().unwrap_or(&"");

        match *cmd {
            "/status" => {
                let spec = self.load_spec()?;
                println!("\n╔═══════════════════════════════════════════════════════════════╗");
                println!("║  Spec Status                                                  ║");
                println!("╚═══════════════════════════════════════════════════════════════╝");
                println!("ID:           {}", spec.spec_id);
                println!("Project:      {}", spec.project);
                println!("Name:         {}", spec.name);
                println!("Stage:        {}", spec.stage);
                println!("Completed:    {:?}", spec.stages_completed);
                println!();
                println!("Requirements: {}", spec.requirements.len());
                for (i, req) in spec.requirements.iter().enumerate().take(5) {
                    println!("  {}: {} - {}", i + 1, req.id, req.title);
                }
                if spec.requirements.len() > 5 {
                    println!("  ... and {} more", spec.requirements.len() - 5);
                }
                println!();
                println!("Decisions:    {}", spec.decisions.len());
                println!("Tasks:        {}", spec.tasks.len());
                println!();
                Ok(false)
            }
            "/show" => {
                let spec = self.load_spec()?;
                let json = serde_json::to_string_pretty(&spec)?;
                println!("\n{}\n", json);
                Ok(false)
            }
            "/advance" => {
                println!("\n🔄 Checking if workflow can advance...");

                // Use workflow engine to check
                let spec = self.load_spec()?;
                match crate::workflow::WorkflowEngine::can_advance(&spec) {
                    Ok(next_stage) => {
                        println!("✓ Can advance to: {}", next_stage);
                        println!("\nAdvancing workflow stage...");

                        // Actually advance
                        match crate::workflow::WorkflowEngine::advance_stage(
                            &spec,
                            next_stage.clone(),
                        ) {
                            Ok(transition) => {
                                // Update spec
                                let mut updated_spec = spec.clone();
                                if !updated_spec.stages_completed.contains(&spec.stage) {
                                    updated_spec.stages_completed.push(spec.stage.clone());
                                }
                                updated_spec.stage = transition.to.clone();
                                updated_spec.history.updated_at = chrono::Utc::now().timestamp();

                                // Log event
                                self.db.log_workflow_event(
                                    &updated_spec.spec_id,
                                    &transition.to.to_string(),
                                    &transition.event.as_string(),
                                    "llm-session",
                                    updated_spec.history.updated_at,
                                    Some(&format!(
                                        "Advanced from {} to {}",
                                        transition.from, transition.to
                                    )),
                                )?;

                                self.db.update_spec(&updated_spec)?;

                                println!("✓ Advanced to stage: {}", transition.to);

                                // Update system prompt with new stage
                                self.init_system_prompt(&updated_spec);
                            }
                            Err(e) => {
                                println!("✗ Failed to advance: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("✗ Cannot advance: {}", e);
                    }
                }
                println!();
                Ok(false)
            }
            "/exit" | "/quit" => {
                println!("\n👋 Exiting LLM editing session...");
                Ok(true)
            }
            _ => {
                println!("Unknown command: {}", cmd);
                println!("Available commands: /status, /show, /advance, /exit");
                println!();
                Ok(false)
            }
        }
    }

    /// Load current spec from database
    fn load_spec(&self) -> Result<SpecData> {
        let spec_row = self
            .db
            .get_spec(&self.spec_id)?
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", self.spec_id))?;

        let spec: SpecData =
            serde_json::from_value(spec_row.data).context("Failed to parse spec data")?;

        Ok(spec)
    }
}
