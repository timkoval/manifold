//! SQLite database layer for manifold
//!
//! Handles all database operations including FTS5 indexing

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::config::ManifoldPaths;
use crate::models::{Boundary, SpecData, SpecRow, WorkflowStage};

/// Database wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open an existing database
    pub fn open(paths: &ManifoldPaths) -> Result<Self> {
        let conn =
            Connection::open(&paths.db_file).context("Failed to open manifold database")?;
        Ok(Self { conn })
    }

    /// Initialize a new database with schema
    pub fn init(paths: &ManifoldPaths) -> Result<Self> {
        let conn =
            Connection::open(&paths.db_file).context("Failed to create manifold database")?;

        // Create main specs table (simplified for LLM-native format)
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS specs (
                id            TEXT PRIMARY KEY,
                project       TEXT NOT NULL,
                boundary      TEXT NOT NULL,
                data          TEXT NOT NULL,
                stage         TEXT NOT NULL DEFAULT 'requirements',
                updated_at    INTEGER NOT NULL,
                created_at    INTEGER NOT NULL
            )
            "#,
            [],
        )
        .context("Failed to create specs table")?;

        // Create FTS5 virtual table for full-text search
        conn.execute(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS specs_fts USING fts5(
                id,
                project,
                boundary,
                name,
                content,
                tokenize = 'unicode61'
            )
            "#,
            [],
        )
        .context("Failed to create FTS5 table")?;

        // Create workflow events table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_events (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                spec_id    TEXT NOT NULL,
                stage      TEXT NOT NULL,
                event      TEXT NOT NULL,
                actor      TEXT NOT NULL,
                timestamp  INTEGER NOT NULL,
                details    TEXT,
                FOREIGN KEY (spec_id) REFERENCES specs(id)
            )
            "#,
            [],
        )
        .context("Failed to create workflow_events table")?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_specs_project ON specs(project)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_specs_boundary ON specs(boundary)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_specs_stage ON specs(stage)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_spec ON workflow_events(spec_id)",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Insert a new spec
    pub fn insert_spec(&self, spec: &SpecData) -> Result<String> {
        let id = spec.spec_id.clone();
        let data_json =
            serde_json::to_string(spec).context("Failed to serialize spec")?;

        self.conn
            .execute(
                r#"
                INSERT INTO specs (id, project, boundary, data, stage, updated_at, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    id,
                    spec.project,
                    spec.boundary.to_string(),
                    data_json,
                    spec.stage.to_string(),
                    spec.history.updated_at,
                    spec.history.created_at
                ],
            )
            .context("Failed to insert spec")?;

        // Index in FTS
        let content = extract_searchable_content(spec);
        self.conn
            .execute(
                "INSERT INTO specs_fts (id, project, boundary, name, content) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, spec.project, spec.boundary.to_string(), spec.name, content],
            )
            .context("Failed to index spec in FTS")?;

        Ok(id)
    }

    /// Update an existing spec
    pub fn update_spec(&self, spec: &SpecData) -> Result<()> {
        let id = &spec.spec_id;
        let data_json = serde_json::to_string(spec).context("Failed to serialize spec")?;

        self.conn
            .execute(
                r#"
                UPDATE specs 
                SET project = ?2, boundary = ?3, data = ?4, stage = ?5, updated_at = ?6
                WHERE id = ?1
                "#,
                params![
                    id,
                    spec.project,
                    spec.boundary.to_string(),
                    data_json,
                    spec.stage.to_string(),
                    spec.history.updated_at
                ],
            )
            .context("Failed to update spec")?;

        // Update FTS index
        self.conn
            .execute(
                "DELETE FROM specs_fts WHERE id = ?1",
                params![id],
            )
            .context("Failed to delete from FTS")?;

        let content = extract_searchable_content(spec);
        self.conn
            .execute(
                "INSERT INTO specs_fts (id, project, boundary, name, content) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, spec.project, spec.boundary.to_string(), spec.name, content],
            )
            .context("Failed to update FTS index")?;

        Ok(())
    }

    /// Get a spec by ID
    pub fn get_spec(&self, id: &str) -> Result<Option<SpecRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project, boundary, data, stage, updated_at, created_at FROM specs WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            let data_str: String = row.get(3)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or_default();
            Ok(SpecRow {
                id: row.get(0)?,
                project: row.get(1)?,
                boundary: row.get(2)?,
                data,
                stage: row.get(4)?,
                updated_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all specs with optional filters
    pub fn list_specs(
        &self,
        boundary: Option<&Boundary>,
        stage: Option<&WorkflowStage>,
    ) -> Result<Vec<SpecRow>> {
        let mut query = String::from(
            "SELECT id, project, boundary, data, stage, updated_at, created_at FROM specs WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(b) = boundary {
            query.push_str(" AND boundary = ?");
            params_vec.push(Box::new(b.to_string()));
        }

        if let Some(s) = stage {
            query.push_str(" AND stage = ?");
            params_vec.push(Box::new(s.to_string()));
        }

        query.push_str(" ORDER BY updated_at DESC");

        let mut stmt = self.conn.prepare(&query)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let data_str: String = row.get(3)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or_default();
            Ok(SpecRow {
                id: row.get(0)?,
                project: row.get(1)?,
                boundary: row.get(2)?,
                data,
                stage: row.get(4)?,
                updated_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut specs = Vec::new();
        for row in rows {
            specs.push(row?);
        }
        Ok(specs)
    }

    /// Search specs using FTS5
    pub fn search_specs(&self, query: &str) -> Result<Vec<SpecRow>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT s.id, s.project, s.boundary, s.data, s.stage, s.updated_at, s.created_at
            FROM specs s
            INNER JOIN specs_fts f ON s.id = f.id
            WHERE specs_fts MATCH ?1
            ORDER BY rank
            "#,
        )?;

        let rows = stmt.query_map(params![query], |row| {
            let data_str: String = row.get(3)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or_default();
            Ok(SpecRow {
                id: row.get(0)?,
                project: row.get(1)?,
                boundary: row.get(2)?,
                data,
                stage: row.get(4)?,
                updated_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut specs = Vec::new();
        for row in rows {
            specs.push(row?);
        }
        Ok(specs)
    }
}

/// Generate a human-readable spec ID like "auric-raptor-torque"
pub fn generate_spec_id(project: &str) -> String {
    let adjectives = ["amber", "azure", "bold", "calm", "dark", "eager", "fair", "gold", "hazy", "keen"];
    let nouns = ["anchor", "beacon", "cipher", "delta", "echo", "flux", "grid", "helix", "iris", "jade"];
    
    let uuid = uuid::Uuid::new_v4();
    let bytes = uuid.as_bytes();
    
    let adj = adjectives[(bytes[0] as usize) % adjectives.len()];
    let noun = nouns[(bytes[1] as usize) % nouns.len()];
    
    // Take first word from project or use a hash
    let suffix = project
        .split('-')
        .next()
        .unwrap_or("spec")
        .chars()
        .take(8)
        .collect::<String>();
    
    format!("{}-{}-{}", adj, noun, suffix)
}

/// Extract searchable text content from a spec (LLM-native format)
fn extract_searchable_content(spec: &SpecData) -> String {
    let mut content = Vec::new();
    content.push(spec.name.clone());
    
    for req in &spec.requirements {
        content.push(req.title.clone());
        content.push(req.shall.clone());
        if let Some(rationale) = &req.rationale {
            content.push(rationale.clone());
        }
        content.extend(req.tags.clone());
        for scenario in &req.scenarios {
            content.push(scenario.name.clone());
            content.extend(scenario.given.clone());
            content.push(scenario.when.clone());
            content.extend(scenario.then.clone());
        }
    }
    
    for task in &spec.tasks {
        content.push(task.title.clone());
        content.push(task.description.clone());
    }
    
    for decision in &spec.decisions {
        content.push(decision.title.clone());
        content.push(decision.context.clone());
        content.push(decision.decision.clone());
    }
    
    content.join(" ")
}
