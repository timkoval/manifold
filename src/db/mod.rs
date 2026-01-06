//! SQLite database layer for manifold
//!
//! Handles all database operations including FTS5 indexing

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::collab::{Conflict, ConflictStatus, Review, ReviewStatus, SyncMetadata, SyncStatus};
use crate::config::ManifoldPaths;
use crate::models::{Boundary, SpecData, SpecRow, WorkflowStage};

/// Database wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open an existing database
    pub fn open(paths: &ManifoldPaths) -> Result<Self> {
        let conn = Connection::open(&paths.db_file).context("Failed to open manifold database")?;
        Ok(Self { conn })
    }

    /// Invalidate cached reads to see changes from other processes (e.g., MCP server)
    /// This should be called before reading data that may have been modified externally
    pub fn invalidate_cache(&self) -> Result<()> {
        // Execute a write statement to force SQLite to release any cached read locks
        // and see the latest committed data from other connections
        self.conn.execute_batch("BEGIN IMMEDIATE; COMMIT;")?;
        Ok(())
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

        // Create sync metadata table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS sync_metadata (
                spec_id           TEXT PRIMARY KEY,
                last_sync_timestamp INTEGER NOT NULL,
                last_sync_hash    TEXT NOT NULL,
                remote_branch     TEXT,
                sync_status       TEXT NOT NULL DEFAULT 'unsynced',
                FOREIGN KEY (spec_id) REFERENCES specs(id)
            )
            "#,
            [],
        )
        .context("Failed to create sync_metadata table")?;

        // Create conflicts table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS conflicts (
                id              TEXT PRIMARY KEY,
                spec_id         TEXT NOT NULL,
                field_path      TEXT NOT NULL,
                local_value     TEXT NOT NULL,
                remote_value    TEXT NOT NULL,
                base_value      TEXT,
                detected_at     INTEGER NOT NULL,
                status          TEXT NOT NULL DEFAULT 'unresolved',
                FOREIGN KEY (spec_id) REFERENCES specs(id)
            )
            "#,
            [],
        )
        .context("Failed to create conflicts table")?;

        // Create reviews table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS reviews (
                id            TEXT PRIMARY KEY,
                spec_id       TEXT NOT NULL,
                requester     TEXT NOT NULL,
                reviewer      TEXT NOT NULL,
                status        TEXT NOT NULL DEFAULT 'pending',
                comment       TEXT,
                requested_at  INTEGER NOT NULL,
                reviewed_at   INTEGER,
                FOREIGN KEY (spec_id) REFERENCES specs(id)
            )
            "#,
            [],
        )
        .context("Failed to create reviews table")?;

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
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_conflicts_spec ON conflicts(spec_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_conflicts_status ON conflicts(status)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reviews_spec ON reviews(spec_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reviews_status ON reviews(status)",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Insert a new spec
    pub fn insert_spec(&self, spec: &SpecData) -> Result<String> {
        let id = spec.spec_id.clone();
        let data_json = serde_json::to_string(spec).context("Failed to serialize spec")?;

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
            .execute("DELETE FROM specs_fts WHERE id = ?1", params![id])
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
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

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

    /// Search specs using FTS5 (full-text search)
    /// This is designed for future CLI/TUI search features
    /// Full-text search across spec data using FTS5
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

    /// Log a workflow event
    pub fn log_workflow_event(
        &self,
        spec_id: &str,
        stage: &str,
        event: &str,
        actor: &str,
        timestamp: i64,
        details: Option<&str>,
    ) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO workflow_events (spec_id, stage, event, actor, timestamp, details)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![spec_id, stage, event, actor, timestamp, details],
            )
            .context("Failed to log workflow event")?;
        Ok(())
    }

    /// Get workflow events for a spec
    pub fn get_workflow_events(&self, spec_id: &str) -> Result<Vec<WorkflowEventRow>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, spec_id, stage, event, actor, timestamp, details
            FROM workflow_events
            WHERE spec_id = ?1
            ORDER BY timestamp DESC
            "#,
        )?;

        let rows = stmt.query_map(params![spec_id], |row| {
            Ok(WorkflowEventRow {
                id: row.get(0)?,
                spec_id: row.get(1)?,
                stage: row.get(2)?,
                event: row.get(3)?,
                actor: row.get(4)?,
                timestamp: row.get(5)?,
                details: row.get(6)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    // Collaboration methods

    /// Save sync metadata for git-based collaboration
    /// Used by sync push/pull to track sync state
    pub fn save_sync_metadata(&self, metadata: &SyncMetadata) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT OR REPLACE INTO sync_metadata 
                (spec_id, last_sync_timestamp, last_sync_hash, remote_branch, sync_status)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    metadata.spec_id,
                    metadata.last_sync_timestamp,
                    metadata.last_sync_hash,
                    metadata.remote_branch,
                    metadata.sync_status.to_string()
                ],
            )
            .context("Failed to save sync metadata")?;
        Ok(())
    }

    /// Get sync metadata for a spec
    /// Used by sync status to show last sync info
    pub fn get_sync_metadata(&self, spec_id: &str) -> Result<Option<SyncMetadata>> {
        let mut stmt = self.conn.prepare(
            "SELECT spec_id, last_sync_timestamp, last_sync_hash, remote_branch, sync_status FROM sync_metadata WHERE spec_id = ?1",
        )?;

        let result = stmt.query_row(params![spec_id], |row| {
            Ok(SyncMetadata {
                spec_id: row.get(0)?,
                last_sync_timestamp: row.get(1)?,
                last_sync_hash: row.get(2)?,
                remote_branch: row.get(3)?,
                sync_status: row
                    .get::<_, String>(4)?
                    .parse()
                    .unwrap_or(SyncStatus::Unsynced),
            })
        });

        match result {
            Ok(metadata) => Ok(Some(metadata)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save conflict
    pub fn save_conflict(&self, conflict: &Conflict) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT OR REPLACE INTO conflicts 
                (id, spec_id, field_path, local_value, remote_value, base_value, detected_at, status)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    conflict.id,
                    conflict.spec_id,
                    conflict.field_path,
                    serde_json::to_string(&conflict.local_value)?,
                    serde_json::to_string(&conflict.remote_value)?,
                    conflict.base_value.as_ref().map(|v| serde_json::to_string(v).ok()).flatten(),
                    conflict.detected_at,
                    conflict.status.to_string()
                ],
            )
            .context("Failed to save conflict")?;
        Ok(())
    }

    /// Get conflicts for a spec
    pub fn get_conflicts(&self, spec_id: &str) -> Result<Vec<Conflict>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, spec_id, field_path, local_value, remote_value, base_value, detected_at, status FROM conflicts WHERE spec_id = ?1 AND status = 'unresolved'",
        )?;

        let rows = stmt.query_map(params![spec_id], |row| {
            let base_value_str: Option<String> = row.get(5)?;
            Ok(Conflict {
                id: row.get(0)?,
                spec_id: row.get(1)?,
                field_path: row.get(2)?,
                local_value: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                remote_value: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                base_value: base_value_str.and_then(|s| serde_json::from_str(&s).ok()),
                detected_at: row.get(6)?,
                status: row
                    .get::<_, String>(7)?
                    .parse()
                    .unwrap_or(ConflictStatus::Unresolved),
            })
        })?;

        let mut conflicts = Vec::new();
        for row in rows {
            conflicts.push(row?);
        }
        Ok(conflicts)
    }

    /// Update conflict status
    pub fn update_conflict_status(&self, conflict_id: &str, status: &ConflictStatus) -> Result<()> {
        self.conn
            .execute(
                "UPDATE conflicts SET status = ?1 WHERE id = ?2",
                params![status.to_string(), conflict_id],
            )
            .context("Failed to update conflict status")?;
        Ok(())
    }

    /// Save review
    pub fn save_review(&self, review: &Review) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT OR REPLACE INTO reviews 
                (id, spec_id, requester, reviewer, status, comment, requested_at, reviewed_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    review.id,
                    review.spec_id,
                    review.requester,
                    review.reviewer,
                    review.status.to_string(),
                    review.comment,
                    review.requested_at,
                    review.reviewed_at
                ],
            )
            .context("Failed to save review")?;
        Ok(())
    }

    /// Get reviews for a spec
    pub fn get_reviews(&self, spec_id: &str) -> Result<Vec<Review>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, spec_id, requester, reviewer, status, comment, requested_at, reviewed_at FROM reviews WHERE spec_id = ?1 ORDER BY requested_at DESC",
        )?;

        let rows = stmt.query_map(params![spec_id], |row| {
            Ok(Review {
                id: row.get(0)?,
                spec_id: row.get(1)?,
                requester: row.get(2)?,
                reviewer: row.get(3)?,
                status: row
                    .get::<_, String>(4)?
                    .parse()
                    .unwrap_or(ReviewStatus::Pending),
                comment: row.get(5)?,
                requested_at: row.get(6)?,
                reviewed_at: row.get(7)?,
            })
        })?;

        let mut reviews = Vec::new();
        for row in rows {
            reviews.push(row?);
        }
        Ok(reviews)
    }

    /// Get review by ID
    pub fn get_review(&self, review_id: &str) -> Result<Option<Review>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, spec_id, requester, reviewer, status, comment, requested_at, reviewed_at FROM reviews WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![review_id], |row| {
            Ok(Review {
                id: row.get(0)?,
                spec_id: row.get(1)?,
                requester: row.get(2)?,
                reviewer: row.get(3)?,
                status: row
                    .get::<_, String>(4)?
                    .parse()
                    .unwrap_or(ReviewStatus::Pending),
                comment: row.get(5)?,
                requested_at: row.get(6)?,
                reviewed_at: row.get(7)?,
            })
        });

        match result {
            Ok(review) => Ok(Some(review)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// Database row for workflow events
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WorkflowEventRow {
    pub id: i64,
    pub spec_id: String,
    pub stage: String,
    pub event: String,
    pub actor: String,
    pub timestamp: i64,
    pub details: Option<String>,
}

/// Generate a human-readable spec ID like "auric-raptor-torque"
pub fn generate_spec_id(project: &str) -> String {
    let adjectives = [
        "amber", "azure", "bold", "calm", "dark", "eager", "fair", "gold", "hazy", "keen",
    ];
    let nouns = [
        "anchor", "beacon", "cipher", "delta", "echo", "flux", "grid", "helix", "iris", "jade",
    ];

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
