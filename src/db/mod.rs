//! SQLite database layer for manifold
//!
//! Handles all database operations including FTS5 indexing

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::collab::{Conflict, ConflictStatus, Review, ReviewStatus, SyncMetadata, SyncStatus};
use crate::config::ManifoldPaths;
use crate::models::{
    Boundary, ManifoldV2, Node, NodeRow, NodeType, SpecData, SpecRow, WorkflowStage,
};

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

        // Create v2 manifolds table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS manifolds (
                manifold_id   TEXT PRIMARY KEY,
                data         TEXT NOT NULL,
                version      INTEGER NOT NULL DEFAULT 1,
                updated_at   INTEGER NOT NULL,
                created_at   INTEGER NOT NULL
            )
            "#,
            [],
        )
        .context("Failed to create manifolds table")?;

        // Create v2 nodes table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS nodes (
                id            TEXT NOT NULL,
                manifold_id   TEXT NOT NULL,
                node_type     TEXT NOT NULL,
                boundary      TEXT NOT NULL,
                title         TEXT,
                content       TEXT,
                links         TEXT,
                updated_at    INTEGER,
                created_at    INTEGER,
                PRIMARY KEY (id, manifold_id),
                FOREIGN KEY (manifold_id) REFERENCES manifolds(manifold_id)
            )
            "#,
            [],
        )
        .context("Failed to create nodes table")?;

        // Create FTS5 virtual table for v2 nodes
        conn.execute(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS nodes_fts USING fts5(
                id,
                manifold_id,
                node_type,
                boundary,
                title,
                content,
                tokenize = 'unicode61'
            )
            "#,
            [],
        )
        .context("Failed to create nodes FTS5 table")?;

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
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_nodes_manifold ON nodes(manifold_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_nodes_boundary ON nodes(boundary)",
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

    // ============================================================================
    // V2 Manifold Operations
    // ============================================================================

    /// Create or update a manifold (v2)
    pub fn upsert_manifold(&self, manifold: &ManifoldV2) -> Result<()> {
        let data_json = serde_json::to_string(manifold).context("Failed to serialize manifold")?;

        let now = chrono::Utc::now().timestamp();

        self.conn
            .execute(
                r#"
                INSERT OR REPLACE INTO manifolds (manifold_id, data, version, updated_at, created_at)
                VALUES (?1, ?2, ?3, ?4, COALESCE((SELECT created_at FROM manifolds WHERE manifold_id = ?1), ?4))
                "#,
                params![
                    manifold.manifold_id,
                    data_json,
                    manifold.version,
                    now
                ],
            )
            .context("Failed to upsert manifold")?;

        Ok(())
    }

    /// Get a manifold by ID (v2)
    pub fn get_manifold(&self, manifold_id: &str) -> Result<Option<ManifoldV2>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM manifolds WHERE manifold_id = ?1")?;

        let result = stmt.query_row(params![manifold_id], |row| {
            let data_str: String = row.get(0)?;
            serde_json::from_str::<ManifoldV2>(&data_str)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))
        });

        match result {
            Ok(manifold) => Ok(Some(manifold)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new node (v2)
    pub fn insert_node(&self, manifold_id: &str, node: &Node) -> Result<()> {
        let content_json = node
            .content
            .as_ref()
            .map(|c| serde_json::to_value(c).ok())
            .flatten();
        let content_str = content_json
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let links_json = serde_json::to_string(&node.links).ok();

        let now = chrono::Utc::now().timestamp();

        // Validate boundary exists in manifold
        let manifold = self
            .get_manifold(manifold_id)?
            .context("Manifold not found")?;

        if !manifold.boundaries.contains_key(&node.boundary) {
            anyhow::bail!(
                "Boundary '{}' is not defined in manifold. Available boundaries: {:?}",
                node.boundary,
                manifold.boundaries.keys().collect::<Vec<_>>()
            );
        }

        self.conn
            .execute(
                r#"
                INSERT INTO nodes (id, manifold_id, node_type, boundary, title, content, links, updated_at, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    node.id,
                    manifold_id,
                    node.node_type.to_string(),
                    node.boundary,
                    node.title.as_deref(),
                    content_str.as_deref(),
                    links_json.as_deref(),
                    now,
                    now
                ],
            )
            .context("Failed to insert node")?;

        // Index in FTS
        let searchable_content = extract_node_searchable_content(node);
        self.conn
            .execute(
                "INSERT INTO nodes_fts (id, manifold_id, node_type, boundary, title, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    node.id,
                    manifold_id,
                    node.node_type.to_string(),
                    node.boundary,
                    node.title.as_deref().unwrap_or(""),
                    searchable_content
                ],
            )
            .context("Failed to index node in FTS")?;

        Ok(())
    }

    /// Update an existing node (v2)
    pub fn update_node(&self, manifold_id: &str, node: &Node) -> Result<()> {
        let content_json = node
            .content
            .as_ref()
            .map(|c| serde_json::to_value(c).ok())
            .flatten();
        let content_str = content_json
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let links_json = serde_json::to_string(&node.links).ok();

        let now = chrono::Utc::now().timestamp();

        self.conn
            .execute(
                r#"
                UPDATE nodes
                SET node_type = ?3, boundary = ?4, title = ?5, content = ?6, links = ?7, updated_at = ?8
                WHERE id = ?1 AND manifold_id = ?2
                "#,
                params![
                    node.id,
                    manifold_id,
                    node.node_type.to_string(),
                    node.boundary,
                    node.title.as_deref(),
                    content_str.as_deref(),
                    links_json.as_deref(),
                    now
                ],
            )
            .context("Failed to update node")?;

        // Update FTS index
        self.conn
            .execute(
                "DELETE FROM nodes_fts WHERE id = ?1 AND manifold_id = ?2",
                params![node.id, manifold_id],
            )
            .context("Failed to delete from FTS")?;

        let searchable_content = extract_node_searchable_content(node);
        self.conn
            .execute(
                "INSERT INTO nodes_fts (id, manifold_id, node_type, boundary, title, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    node.id,
                    manifold_id,
                    node.node_type.to_string(),
                    node.boundary,
                    node.title.as_deref().unwrap_or(""),
                    searchable_content
                ],
            )
            .context("Failed to update FTS index")?;

        Ok(())
    }

    /// Get a node by ID (v2)
    pub fn get_node(&self, manifold_id: &str, node_id: &str) -> Result<Option<Node>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, node_type, boundary, title, content, links, updated_at, created_at FROM nodes WHERE id = ?1 AND manifold_id = ?2",
        )?;

        let result = stmt.query_row(params![node_id, manifold_id], |row| {
            let node_type_str: String = row.get(1)?;
            let node_type = node_type_str.parse().unwrap_or(NodeType::Spec);
            let content_str: Option<String> = row.get(4)?;
            let links_str: Option<String> = row.get(5)?;

            let content = content_str.and_then(|s| serde_json::from_str(&s).ok());
            let links: Vec<String> = links_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            Ok(Node {
                id: row.get(0)?,
                node_type,
                boundary: row.get(2)?,
                title: row.get(3)?,
                content,
                links,
                history: None, // Will be populated from content if needed
                embeddings: None,
            })
        });

        match result {
            Ok(node) => Ok(Some(node)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List nodes in a manifold with optional filters (v2)
    pub fn list_nodes(
        &self,
        manifold_id: &str,
        node_type: Option<&NodeType>,
        boundary: Option<&str>,
    ) -> Result<Vec<NodeRow>> {
        let mut query = String::from(
            "SELECT id, manifold_id, node_type, boundary, title, content, links, updated_at, created_at FROM nodes WHERE manifold_id = ?1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(manifold_id.to_string())];

        if node_type.is_some() || boundary.is_some() {
            if let Some(nt) = node_type {
                query.push_str(" AND node_type = ?");
                params_vec.push(Box::new(nt.to_string()));
            }
            if let Some(b) = boundary {
                query.push_str(" AND boundary = ?");
                params_vec.push(Box::new(b.to_string()));
            }
        }

        query.push_str(" ORDER BY updated_at DESC");

        let mut stmt = self.conn.prepare(&query)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let content_str: Option<String> = row.get(5)?;
            let content = content_str.and_then(|s| serde_json::from_str(&s).ok());
            let links_str: Option<String> = row.get(6)?;
            let links: Vec<String> = links_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            Ok(NodeRow {
                id: row.get(0)?,
                manifold_id: row.get(1)?,
                node_type: row.get(2)?,
                boundary: row.get(3)?,
                title: row.get(4)?,
                content,
                links,
                updated_at: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut nodes = Vec::new();
        for row in rows {
            nodes.push(row?);
        }
        Ok(nodes)
    }

    /// Search nodes using FTS5 (v2)
    pub fn search_nodes(&self, manifold_id: &str, query: &str) -> Result<Vec<NodeRow>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT n.id, n.manifold_id, n.node_type, n.boundary, n.title, n.content, n.links, n.updated_at, n.created_at
            FROM nodes n
            INNER JOIN nodes_fts f ON n.id = f.id AND n.manifold_id = f.manifold_id
            WHERE n.manifold_id = ?1 AND nodes_fts MATCH ?2
            ORDER BY rank
            "#,
        )?;

        let rows = stmt.query_map(params![manifold_id, query], |row| {
            let content_str: Option<String> = row.get(5)?;
            let content = content_str.and_then(|s| serde_json::from_str(&s).ok());
            let links_str: Option<String> = row.get(6)?;
            let links: Vec<String> = links_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            Ok(NodeRow {
                id: row.get(0)?,
                manifold_id: row.get(1)?,
                node_type: row.get(2)?,
                boundary: row.get(3)?,
                title: row.get(4)?,
                content,
                links,
                updated_at: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut nodes = Vec::new();
        for row in rows {
            nodes.push(row?);
        }
        Ok(nodes)
    }

    /// Validate node against manifold boundaries
    pub fn validate_node_boundary(&self, manifold_id: &str, node: &Node) -> Result<bool> {
        let manifold = self.get_manifold(manifold_id)?;

        match manifold {
            Some(m) => {
                let valid = m.boundaries.contains_key(&node.boundary);
                if !valid {
                    anyhow::bail!(
                        "Boundary '{}' is not defined in manifold. Available: {:?}",
                        node.boundary,
                        m.boundaries.keys().collect::<Vec<_>>()
                    );
                }
                Ok(valid)
            }
            None => {
                anyhow::bail!("Manifold '{}' not found", manifold_id);
            }
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

/// Extract searchable text content from a v2 node
fn extract_node_searchable_content(node: &Node) -> String {
    let mut content = Vec::new();

    if let Some(title) = &node.title {
        content.push(title.clone());
    }

    if let Some(node_content) = &node.content {
        match node_content {
            crate::models::NodeContent::Project(proj) => {
                if let Some(name) = &proj.name {
                    content.push(name.clone());
                }
                if let Some(desc) = &proj.description {
                    content.push(desc.clone());
                }
                for req in &proj.requirements {
                    content.push(req.title.clone());
                    content.push(req.shall.clone());
                }
                for task in &proj.tasks {
                    content.push(task.title.clone());
                    content.push(task.description.clone());
                }
            }
            crate::models::NodeContent::Spec(spec) => {
                for req in &spec.requirements {
                    content.push(req.title.clone());
                    content.push(req.shall.clone());
                }
                for task in &spec.tasks {
                    content.push(task.title.clone());
                    content.push(task.description.clone());
                }
            }
            crate::models::NodeContent::Knowledge(k) => {
                content.push(k.topic.clone());
                content.push(k.notes.clone());
                content.extend(k.tags.clone());
            }
            crate::models::NodeContent::Diary(d) => {
                content.push(d.date.clone());
                content.push(d.reflection.clone());
            }
            crate::models::NodeContent::Research(r) => {
                content.push(r.hub.clone());
                for entry in &r.entries {
                    content.push(entry.source.clone());
                    content.push(entry.summary.clone());
                }
            }
        }
    }

    content.join(" ")
}
