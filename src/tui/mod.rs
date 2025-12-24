//! Terminal UI for manifold using ratatui
//!
//! Provides an interactive dashboard for browsing and managing specs

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::io;

use crate::collab::conflicts::ConflictResolver;
use crate::collab::{Conflict, ConflictStatus, ResolutionStrategy};
use crate::config::ManifoldPaths;
use crate::db::Database;
use crate::models::{SpecData, SpecRow};

/// Main TUI application state
pub struct TuiApp {
    db: Database,
    specs: Vec<SpecRow>,
    list_state: ListState,
    selected_tab: usize,
    should_quit: bool,
    filter_boundary: Option<String>,
    conflicts: Vec<Conflict>,
    conflict_list_state: ListState,
    show_resolution_popup: bool,
    selected_strategy: usize,
    status_message: Option<String>,
    // Manual editing state
    show_manual_edit_popup: bool,
    manual_edit_input: String,
    // Bulk operations
    show_bulk_popup: bool,
    // Conflict statistics
    conflict_stats: ConflictStats,
}

#[derive(Default, Clone)]
struct ConflictStats {
    total: usize,
    unresolved: usize,
    resolved: usize,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(paths: &ManifoldPaths) -> Result<Self> {
        let db = Database::open(paths)?;
        let specs = db.list_specs(None, None)?;
        
        let mut list_state = ListState::default();
        if !specs.is_empty() {
            list_state.select(Some(0));
        }

        let conflict_list_state = ListState::default();

        Ok(Self {
            db,
            specs,
            list_state,
            selected_tab: 0,
            should_quit: false,
            filter_boundary: None,
            conflicts: Vec::new(),
            conflict_list_state,
            show_resolution_popup: false,
            selected_strategy: 0,
            status_message: None,
            show_manual_edit_popup: false,
            manual_edit_input: String::new(),
            show_bulk_popup: false,
            conflict_stats: ConflictStats::default(),
        })
    }

    /// Run the TUI application
    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run event loop
        let res = self.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        res
    }

    /// Main event loop
    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Handle popup-specific keys first
                    if self.show_resolution_popup || self.show_bulk_popup || self.show_manual_edit_popup {
                        match key.code {
                            KeyCode::Esc => {
                                self.show_resolution_popup = false;
                                self.show_bulk_popup = false;
                                self.show_manual_edit_popup = false;
                                continue;
                            }
                            KeyCode::Enter if self.show_resolution_popup => {
                                if self.selected_strategy == 3 {
                                    self.show_resolution_popup = false;
                                    self.show_manual_edit_popup = true;
                                    self.manual_edit_input.clear();
                                } else {
                                    self.apply_resolution()?;
                                }
                                continue;
                            }
                            KeyCode::Enter if self.show_bulk_popup => {
                                self.apply_bulk_resolution()?;
                                continue;
                            }
                            KeyCode::Enter if self.show_manual_edit_popup => {
                                self.apply_manual_resolution()?;
                                continue;
                            }
                            KeyCode::Char(c) if self.show_manual_edit_popup => {
                                self.manual_edit_input.push(c);
                                continue;
                            }
                            KeyCode::Backspace if self.show_manual_edit_popup => {
                                self.manual_edit_input.pop();
                                continue;
                            }
                            KeyCode::Left if self.show_resolution_popup || self.show_bulk_popup => {
                                if self.selected_strategy > 0 {
                                    self.selected_strategy -= 1;
                                }
                                continue;
                            }
                            KeyCode::Right if self.show_resolution_popup || self.show_bulk_popup => {
                                if self.selected_strategy < 3 {
                                    self.selected_strategy += 1;
                                }
                                continue;
                            }
                            _ => continue,
                        }
                    }

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.should_quit = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.should_quit = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.next_spec();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.previous_spec();
                        }
                        KeyCode::Tab => {
                            self.next_tab();
                        }
                        KeyCode::BackTab => {
                            self.previous_tab();
                        }
                        KeyCode::Char('r') => {
                            // Refresh
                            self.refresh_specs()?;
                        }
                        KeyCode::Char('1') => {
                            self.filter_boundary = None;
                            self.refresh_specs()?;
                        }
                        KeyCode::Char('2') => {
                            self.filter_boundary = Some("personal".to_string());
                            self.refresh_specs()?;
                        }
                        KeyCode::Char('3') => {
                            self.filter_boundary = Some("work".to_string());
                            self.refresh_specs()?;
                        }
                        KeyCode::Char('4') => {
                            self.filter_boundary = Some("company".to_string());
                            self.refresh_specs()?;
                        }
                        KeyCode::Char('c') if self.selected_tab == 5 => {
                            // Load conflicts for selected spec
                            self.load_conflicts()?;
                        }
                        KeyCode::Char('o') if self.selected_tab == 5 && !self.show_resolution_popup => {
                            // Open resolution popup
                            if self.conflict_list_state.selected().is_some() {
                                self.show_resolution_popup = true;
                                self.selected_strategy = 0;
                            }
                        }
                        KeyCode::Char('b') if self.selected_tab == 5 && !self.show_bulk_popup => {
                            // Open bulk resolution popup
                            if !self.conflicts.is_empty() {
                                self.show_bulk_popup = true;
                                self.selected_strategy = 0;
                            }
                        }
                        KeyCode::Char('a') if self.selected_tab == 5 => {
                            // Auto-merge all compatible conflicts
                            self.auto_merge_conflicts()?;
                        }
                        _ => {}
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Draw the UI
    fn ui(&mut self, f: &mut Frame) {
        // Main layout: header, content, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),      // Content
                Constraint::Length(3),  // Footer
            ])
            .split(f.area());

        // Header
        self.render_header(f, chunks[0]);

        // Content area - split into list and detail
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),  // Spec list
                Constraint::Percentage(70),  // Detail view
            ])
            .split(chunks[1]);

        // Render spec list
        self.render_spec_list(f, content_chunks[0]);

        // Render detail view
        self.render_detail_view(f, content_chunks[1]);

        // Footer with keybindings
        self.render_footer(f, chunks[2]);
    }

    /// Render header
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("╔═══════════════════════════════════════════════════════════════╗\n\
                                     ║  Manifold Dashboard - Specification Management System         ║\n\
                                     ╚═══════════════════════════════════════════════════════════════╝")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        f.render_widget(title, area);
    }

    /// Render spec list
    fn render_spec_list(&mut self, f: &mut Frame, area: Rect) {
        let filter_text = match &self.filter_boundary {
            Some(b) => format!(" [{}]", b),
            None => " [all]".to_string(),
        };

        let items: Vec<ListItem> = self
            .specs
            .iter()
            .map(|spec| {
                let stage_icon = match spec.stage.as_str() {
                    "requirements" => "📋",
                    "design" => "📐",
                    "tasks" => "📝",
                    "approval" => "✅",
                    "implemented" => "🎉",
                    _ => "❓",
                };

                let name = spec.data.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&spec.project);

                let content = format!("{} {} ({})", stage_icon, name, spec.boundary);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Specs{} ({})", filter_text, self.specs.len()))
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render detail view
    fn render_detail_view(&mut self, f: &mut Frame, area: Rect) {
        // Tabs
        let tabs_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let titles = vec!["Overview", "Requirements", "Tasks", "Decisions", "History", "Conflicts"];
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Details"))
            .select(self.selected_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_widget(tabs, tabs_area[0]);

        // Tab content
        if let Some(selected) = self.list_state.selected() {
            if let Some(spec_row) = self.specs.get(selected) {
                let content_area = tabs_area[1];
                
                match self.selected_tab {
                    0 => self.render_overview(f, content_area, spec_row),
                    1 => self.render_requirements(f, content_area, spec_row),
                    2 => self.render_tasks(f, content_area, spec_row),
                    3 => self.render_decisions(f, content_area, spec_row),
                    4 => self.render_history(f, content_area, spec_row),
                    5 => self.render_conflicts(f, content_area),
                    _ => {}
                }
            }
        } else {
            let empty = Paragraph::new("No spec selected")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, tabs_area[1]);
        }

        // Show resolution popup if active
        if self.show_resolution_popup {
            self.render_resolution_popup(f);
        }

        // Show bulk popup if active
        if self.show_bulk_popup {
            self.render_bulk_popup(f);
        }

        // Show manual edit popup if active
        if self.show_manual_edit_popup {
            self.render_manual_edit_popup(f);
        }

        // Show status message if present
        if let Some(msg) = &self.status_message {
            self.render_status_message(f, msg);
        }
    }

    /// Render overview tab
    fn render_overview(&self, f: &mut Frame, area: Rect, spec_row: &SpecRow) {
        let spec: SpecData = serde_json::from_value(spec_row.data.clone()).unwrap();
        
        let workflow_stages = ["requirements", "design", "tasks", "approval", "implemented"];
        let current_stage_idx = workflow_stages.iter().position(|&s| s == spec.stage.to_string()).unwrap_or(0);
        
        let mut workflow_viz = String::new();
        for (i, stage) in workflow_stages.iter().enumerate() {
            if i == current_stage_idx {
                workflow_viz.push_str(&format!(" [{}] ", stage.to_uppercase()));
            } else if i < current_stage_idx {
                workflow_viz.push_str(&format!(" ✓ {} ", stage));
            } else {
                workflow_viz.push_str(&format!(" · {} ", stage));
            }
            if i < workflow_stages.len() - 1 {
                workflow_viz.push_str("→");
            }
        }

        let text = format!(
            "Spec ID:      {}\n\
             Project:      {}\n\
             Name:         {}\n\
             Boundary:     {}\n\
             \n\
             Workflow:\n\
             {}\n\
             \n\
             Content:\n\
             Requirements: {}\n\
             Tasks:        {}\n\
             Decisions:    {}\n\
             \n\
             Created:      {}\n\
             Updated:      {}",
            spec.spec_id,
            spec.project,
            spec.name,
            spec.boundary,
            workflow_viz,
            spec.requirements.len(),
            spec.tasks.len(),
            spec.decisions.len(),
            chrono::DateTime::from_timestamp(spec.history.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            chrono::DateTime::from_timestamp(spec.history.updated_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        );

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Render requirements tab
    fn render_requirements(&self, f: &mut Frame, area: Rect, spec_row: &SpecRow) {
        let spec: SpecData = serde_json::from_value(spec_row.data.clone()).unwrap();
        
        let mut text = String::new();
        if spec.requirements.is_empty() {
            text.push_str("No requirements defined yet.\n");
        } else {
            for req in &spec.requirements {
                text.push_str(&format!("\n{} - {} [{}]\n", req.id, req.title, req.priority));
                text.push_str(&format!("SHALL: {}\n", req.shall));
                if let Some(rationale) = &req.rationale {
                    text.push_str(&format!("Rationale: {}\n", rationale));
                }
                if !req.scenarios.is_empty() {
                    text.push_str(&format!("Scenarios: {}\n", req.scenarios.len()));
                }
                text.push_str(&"-".repeat(60));
                text.push('\n');
            }
        }

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Requirements"))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Render tasks tab
    fn render_tasks(&self, f: &mut Frame, area: Rect, spec_row: &SpecRow) {
        let spec: SpecData = serde_json::from_value(spec_row.data.clone()).unwrap();
        
        let mut text = String::new();
        if spec.tasks.is_empty() {
            text.push_str("No tasks defined yet.\n");
        } else {
            for task in &spec.tasks {
                text.push_str(&format!("\n{} - {} [{}]\n", task.id, task.title, task.status));
                text.push_str(&format!("{}\n", task.description));
                text.push_str(&format!("Traces to: {}\n", task.requirement_ids.join(", ")));
                text.push_str(&"-".repeat(60));
                text.push('\n');
            }
        }

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Tasks"))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Render decisions tab
    fn render_decisions(&self, f: &mut Frame, area: Rect, spec_row: &SpecRow) {
        let spec: SpecData = serde_json::from_value(spec_row.data.clone()).unwrap();
        
        let mut text = String::new();
        if spec.decisions.is_empty() {
            text.push_str("No design decisions documented yet.\n");
        } else {
            for decision in &spec.decisions {
                text.push_str(&format!("\n{} - {}\n", decision.id, decision.title));
                text.push_str(&format!("Decision: {}\n", decision.decision));
                text.push_str(&format!("Rationale: {}\n", decision.rationale));
                text.push_str(&format!("Date: {}\n", decision.date));
                text.push_str(&"-".repeat(60));
                text.push('\n');
            }
        }

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Design Decisions"))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Render history tab
    fn render_history(&self, f: &mut Frame, area: Rect, spec_row: &SpecRow) {
        let spec: SpecData = serde_json::from_value(spec_row.data.clone()).unwrap();
        
        let mut text = String::new();
        if spec.history.patches.is_empty() {
            text.push_str("No history recorded.\n");
        } else {
            for patch in spec.history.patches.iter().rev().take(20) {
                let timestamp = chrono::DateTime::from_timestamp(patch.timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                text.push_str(&format!("{} | {} | {}\n  {}\n\n",
                    timestamp, patch.actor, patch.op, patch.summary));
            }
        }

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Change History"))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Render footer with keybindings
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer_text = if self.selected_tab == 5 {
            if self.conflict_stats.total > 0 {
                format!("  ↑/↓: Nav  c: Load  o: Resolve  b: Bulk  a: Auto-merge  r: Refresh  q: Quit  │  {}/{} unresolved",
                    self.conflict_stats.unresolved, self.conflict_stats.total)
            } else {
                "  ↑/↓: Navigate  c: Load Conflicts  o: Resolve  b: Bulk  a: Auto-merge  r: Refresh  q/Esc: Quit".to_string()
            }
        } else {
            "  ↑/↓: Navigate  Tab: Switch Tab  1-4: Filter Boundary  r: Refresh  q/Esc: Quit".to_string()
        };

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(footer, area);
    }

    /// Navigate to next spec
    fn next_spec(&mut self) {
        if self.specs.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.specs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Navigate to previous spec
    fn previous_spec(&mut self) {
        if self.specs.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.specs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Navigate to next tab
    fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % 6;
        self.status_message = None;
    }

    /// Navigate to previous tab
    fn previous_tab(&mut self) {
        if self.selected_tab == 0 {
            self.selected_tab = 5;
        } else {
            self.selected_tab -= 1;
        }
        self.status_message = None;
    }

    /// Refresh spec list from database
    fn refresh_specs(&mut self) -> Result<()> {
        let boundary = self.filter_boundary.as_ref().and_then(|b| b.parse().ok());
        self.specs = self.db.list_specs(boundary.as_ref(), None)?;
        
        if !self.specs.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
        
        Ok(())
    }

    /// Render conflicts tab
    fn render_conflicts(&mut self, f: &mut Frame, area: Rect) {
        if self.conflicts.is_empty() {
            let text = "No conflicts for this spec.\n\nPress 'c' to load conflicts from database.";
            let paragraph = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Conflicts"))
                .style(Style::default().fg(Color::Green))
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
            return;
        }

        // Split area into list and detail
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),  // Conflict list
                Constraint::Percentage(60),  // Conflict detail
            ])
            .split(area);

        // Render conflict list
        let items: Vec<ListItem> = self.conflicts.iter().enumerate().map(|(i, conflict)| {
            let status_icon = match conflict.status {
                ConflictStatus::Unresolved => "⚠",
                ConflictStatus::ResolvedLocal => "✓",
                ConflictStatus::ResolvedRemote => "✓",
                ConflictStatus::ResolvedManual => "✓",
            };
            let content = format!("{} {} - {}", status_icon, i + 1, conflict.field_path);
            let style = match conflict.status {
                ConflictStatus::Unresolved => Style::default().fg(Color::Red),
                _ => Style::default().fg(Color::Green),
            };
            ListItem::new(content).style(style)
        }).collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Conflicts ({})", self.conflicts.len()))
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[0], &mut self.conflict_list_state);

        // Render conflict detail
        if let Some(selected) = self.conflict_list_state.selected() {
            if let Some(conflict) = self.conflicts.get(selected) {
                // Format values with diff highlighting
                let local_val = format_conflict_value(&conflict.local_value);
                let remote_val = format_conflict_value(&conflict.remote_value);
                let base_val = conflict.base_value.as_ref()
                    .map(|v| format_conflict_value(v))
                    .unwrap_or_else(|| "(no base)".to_string());

                // Create diff markers
                let (local_marker, remote_marker) = if local_val != remote_val {
                    ("← LOCAL (different)", "→ REMOTE (different)")
                } else {
                    ("LOCAL", "REMOTE")
                };

                let detail_text = format!(
                    "Conflict ID: {}\n\
                     Field Path: {}\n\
                     Status: {}\n\
                     Detected: {}\n\
                     \n\
                     BASE VALUE:\n\
                     {}\n\
                     \n\
                     {} {}\n\
                     {}\n\
                     \n\
                     {} {}\n\
                     {}\n\
                     \n\
                     Press 'o' to open resolution dialog | 'b' for bulk resolution",
                    conflict.id,
                    conflict.field_path,
                    conflict.status,
                    chrono::DateTime::from_timestamp(conflict.detected_at, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    base_val,
                    "─".repeat(30), local_marker,
                    local_val,
                    "─".repeat(30), remote_marker,
                    remote_val,
                );

                let paragraph = Paragraph::new(detail_text)
                    .block(Block::default().borders(Borders::ALL).title("Conflict Details"))
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(Color::Yellow));

                f.render_widget(paragraph, chunks[1]);
            }
        }
    }

    /// Render resolution popup
    fn render_resolution_popup(&self, f: &mut Frame) {
        let area = centered_rect(60, 40, f.area());

        // Clear background
        let clear = Block::default()
            .style(Style::default().bg(Color::Black));
        f.render_widget(clear, area);

        // Split into title and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Select Resolution Strategy")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Strategies
        let strategies = vec!["Ours (Keep Local)", "Theirs (Accept Remote)", "Merge (Auto)", "Manual"];
        let items: Vec<ListItem> = strategies.iter().enumerate().map(|(i, s)| {
            let style = if i == self.selected_strategy {
                Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(*s).style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Strategies"));
        f.render_widget(list, chunks[1]);

        // Instructions
        let instructions = Paragraph::new("←/→: Select  Enter: Apply  Esc: Cancel")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(instructions, chunks[2]);
    }

    /// Render status message
    fn render_status_message(&self, f: &mut Frame, message: &str) {
        let area = centered_rect(50, 20, f.area());
        
        let paragraph = Paragraph::new(message)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(Color::Green))
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }

    /// Load conflicts for the selected spec
    fn load_conflicts(&mut self) -> Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(spec_row) = self.specs.get(selected) {
                self.conflicts = self.db.get_conflicts(&spec_row.id)?;
                
                // Update statistics
                self.conflict_stats.total = self.conflicts.len();
                self.conflict_stats.unresolved = self.conflicts.iter()
                    .filter(|c| matches!(c.status, ConflictStatus::Unresolved))
                    .count();
                self.conflict_stats.resolved = self.conflict_stats.total - self.conflict_stats.unresolved;
                
                if !self.conflicts.is_empty() {
                    self.conflict_list_state.select(Some(0));
                    self.status_message = Some(format!("Loaded {} conflict(s) ({} unresolved)", 
                        self.conflict_stats.total, self.conflict_stats.unresolved));
                } else {
                    self.conflict_list_state.select(None);
                    self.status_message = Some("No conflicts found for this spec".to_string());
                }
            }
        }
        Ok(())
    }

    /// Apply selected resolution strategy
    fn apply_resolution(&mut self) -> Result<()> {
        if let Some(conflict_idx) = self.conflict_list_state.selected() {
            if let Some(conflict) = self.conflicts.get(conflict_idx) {
                let strategy = match self.selected_strategy {
                    0 => ResolutionStrategy::Ours,
                    1 => ResolutionStrategy::Theirs,
                    2 => ResolutionStrategy::Merge,
                    3 => ResolutionStrategy::Manual,
                    _ => ResolutionStrategy::Ours,
                };

                match ConflictResolver::resolve_conflict(conflict, strategy, None) {
                    Ok((resolved_value, status)) => {
                        // Update conflict status in database
                        self.db.update_conflict_status(&conflict.id, &status)?;
                        
                        // Apply resolution to spec
                        if let Some(selected) = self.list_state.selected() {
                            if let Some(spec_row) = self.specs.get(selected) {
                                let mut spec: SpecData = serde_json::from_value(spec_row.data.clone())?;
                                ConflictResolver::apply_resolutions(&mut spec, &[(conflict.field_path.clone(), resolved_value)])?;
                                self.db.update_spec(&spec)?;
                            }
                        }

                        self.show_resolution_popup = false;
                        self.status_message = Some(format!("✓ Conflict resolved with strategy: {}", 
                            match strategy {
                                ResolutionStrategy::Ours => "ours",
                                ResolutionStrategy::Theirs => "theirs",
                                ResolutionStrategy::Merge => "merge",
                                ResolutionStrategy::Manual => "manual",
                            }
                        ));

                        // Reload conflicts
                        self.load_conflicts()?;
                    }
                    Err(e) => {
                        self.show_resolution_popup = false;
                        self.status_message = Some(format!("✗ Failed to resolve: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    /// Render bulk resolution popup
    fn render_bulk_popup(&self, f: &mut Frame) {
        let area = centered_rect(60, 40, f.area());

        // Clear background
        let clear = Block::default()
            .style(Style::default().bg(Color::Black));
        f.render_widget(clear, area);

        // Split into title and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Bulk Resolution - Resolve All Unresolved Conflicts")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Strategies
        let strategies = vec!["Ours (Keep Local)", "Theirs (Accept Remote)", "Merge (Auto)", "Manual"];
        let items: Vec<ListItem> = strategies.iter().enumerate().map(|(i, s)| {
            let style = if i == self.selected_strategy {
                Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(*s).style(style)
        }).collect();

        let unresolved_count = self.conflicts.iter()
            .filter(|c| matches!(c.status, ConflictStatus::Unresolved))
            .count();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL)
                .title(format!("Strategy (will apply to {} conflicts)", unresolved_count)));
        f.render_widget(list, chunks[1]);

        // Instructions
        let instructions = Paragraph::new("←/→: Select  Enter: Apply to All  Esc: Cancel")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(instructions, chunks[2]);
    }

    /// Render manual edit popup
    fn render_manual_edit_popup(&self, f: &mut Frame) {
        let area = centered_rect(70, 50, f.area());

        // Clear background
        let clear = Block::default()
            .style(Style::default().bg(Color::Black));
        f.render_widget(clear, area);

        // Split into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(10),
                Constraint::Length(5),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Manual Value Entry")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Show current values for context
        if let Some(conflict_idx) = self.conflict_list_state.selected() {
            if let Some(conflict) = self.conflicts.get(conflict_idx) {
                let context = format!(
                    "Field: {}\n\
                     Local:  {}\n\
                     Remote: {}\n\
                     \n\
                     Enter your custom value below:",
                    conflict.field_path,
                    format_conflict_value(&conflict.local_value),
                    format_conflict_value(&conflict.remote_value),
                );
                let paragraph = Paragraph::new(context)
                    .block(Block::default().borders(Borders::ALL).title("Context"))
                    .wrap(Wrap { trim: true });
                f.render_widget(paragraph, chunks[1]);
            }
        }

        // Input field
        let input = Paragraph::new(self.manual_edit_input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Custom Value"))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(input, chunks[2]);

        // Instructions
        let instructions = Paragraph::new("Type: Enter value  Enter: Apply  Esc: Cancel")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(instructions, chunks[3]);
    }

    /// Apply bulk resolution to all unresolved conflicts
    fn apply_bulk_resolution(&mut self) -> Result<()> {
        let strategy = match self.selected_strategy {
            0 => ResolutionStrategy::Ours,
            1 => ResolutionStrategy::Theirs,
            2 => ResolutionStrategy::Merge,
            3 => ResolutionStrategy::Manual,
            _ => ResolutionStrategy::Ours,
        };

        if matches!(strategy, ResolutionStrategy::Manual) {
            self.show_bulk_popup = false;
            self.status_message = Some("✗ Manual strategy not supported for bulk resolution".to_string());
            return Ok(());
        }

        let mut resolved_count = 0;
        let mut failed_count = 0;
        let mut resolutions = Vec::new();

        // Collect unresolved conflicts
        let unresolved_conflicts: Vec<_> = self.conflicts.iter()
            .filter(|c| matches!(c.status, ConflictStatus::Unresolved))
            .cloned()
            .collect();

        for conflict in &unresolved_conflicts {
            match ConflictResolver::resolve_conflict(conflict, strategy, None) {
                Ok((resolved_value, status)) => {
                    // Update conflict status in database
                    if let Err(e) = self.db.update_conflict_status(&conflict.id, &status) {
                        failed_count += 1;
                        eprintln!("Failed to update conflict {}: {}", conflict.id, e);
                        continue;
                    }
                    resolutions.push((conflict.field_path.clone(), resolved_value));
                    resolved_count += 1;
                }
                Err(_) => {
                    failed_count += 1;
                }
            }
        }

        // Apply all resolutions to spec
        if !resolutions.is_empty() {
            if let Some(selected) = self.list_state.selected() {
                if let Some(spec_row) = self.specs.get(selected) {
                    let mut spec: SpecData = serde_json::from_value(spec_row.data.clone())?;
                    if let Err(e) = ConflictResolver::apply_resolutions(&mut spec, &resolutions) {
                        self.show_bulk_popup = false;
                        self.status_message = Some(format!("✗ Failed to apply resolutions: {}", e));
                        return Ok(());
                    }
                    self.db.update_spec(&spec)?;
                }
            }
        }

        self.show_bulk_popup = false;
        self.status_message = Some(format!(
            "✓ Bulk resolution complete: {} resolved, {} failed",
            resolved_count, failed_count
        ));

        // Reload conflicts
        self.load_conflicts()?;

        Ok(())
    }

    /// Apply manual resolution with custom value
    fn apply_manual_resolution(&mut self) -> Result<()> {
        if let Some(conflict_idx) = self.conflict_list_state.selected() {
            if let Some(conflict) = self.conflicts.get(conflict_idx) {
                // Parse input as JSON value
                let manual_value: serde_json::Value = if self.manual_edit_input.trim().is_empty() {
                    serde_json::Value::Null
                } else {
                    // Try parsing as JSON, otherwise treat as string
                    serde_json::from_str(&self.manual_edit_input)
                        .unwrap_or_else(|_| serde_json::Value::String(self.manual_edit_input.clone()))
                };

                match ConflictResolver::resolve_conflict(conflict, ResolutionStrategy::Manual, Some(manual_value.clone())) {
                    Ok((resolved_value, status)) => {
                        // Update conflict status in database
                        self.db.update_conflict_status(&conflict.id, &status)?;
                        
                        // Apply resolution to spec
                        if let Some(selected) = self.list_state.selected() {
                            if let Some(spec_row) = self.specs.get(selected) {
                                let mut spec: SpecData = serde_json::from_value(spec_row.data.clone())?;
                                ConflictResolver::apply_resolutions(&mut spec, &[(conflict.field_path.clone(), resolved_value)])?;
                                self.db.update_spec(&spec)?;
                            }
                        }

                        self.show_manual_edit_popup = false;
                        self.status_message = Some("✓ Manual value applied successfully".to_string());

                        // Reload conflicts
                        self.load_conflicts()?;
                    }
                    Err(e) => {
                        self.show_manual_edit_popup = false;
                        self.status_message = Some(format!("✗ Failed to apply manual value: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    /// Auto-merge all compatible conflicts
    fn auto_merge_conflicts(&mut self) -> Result<()> {
        let mut merged_count = 0;
        let mut failed_count = 0;
        let mut skipped_count = 0;
        let mut resolutions = Vec::new();

        // Try to auto-merge all unresolved conflicts
        let unresolved_conflicts: Vec<_> = self.conflicts.iter()
            .filter(|c| matches!(c.status, ConflictStatus::Unresolved))
            .cloned()
            .collect();

        for conflict in &unresolved_conflicts {
            match ConflictResolver::resolve_conflict(conflict, ResolutionStrategy::Merge, None) {
                Ok((resolved_value, status)) => {
                    // Update conflict status in database
                    if let Err(e) = self.db.update_conflict_status(&conflict.id, &status) {
                        failed_count += 1;
                        eprintln!("Failed to update conflict {}: {}", conflict.id, e);
                        continue;
                    }
                    resolutions.push((conflict.field_path.clone(), resolved_value));
                    merged_count += 1;
                }
                Err(_) => {
                    // Cannot auto-merge, requires manual resolution
                    skipped_count += 1;
                }
            }
        }

        // Apply all resolutions to spec
        if !resolutions.is_empty() {
            if let Some(selected) = self.list_state.selected() {
                if let Some(spec_row) = self.specs.get(selected) {
                    let mut spec: SpecData = serde_json::from_value(spec_row.data.clone())?;
                    if let Err(e) = ConflictResolver::apply_resolutions(&mut spec, &resolutions) {
                        self.status_message = Some(format!("✗ Failed to apply auto-merge: {}", e));
                        return Ok(());
                    }
                    self.db.update_spec(&spec)?;
                }
            }
        }

        self.status_message = Some(format!(
            "✓ Auto-merge: {} merged, {} skipped (need manual), {} failed",
            merged_count, skipped_count, failed_count
        ));

        // Reload conflicts
        self.load_conflicts()?;

        Ok(())
    }
}

/// Helper function to format JSON value for display
fn format_conflict_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "(deleted)".to_string(),
        serde_json::Value::Object(_) => serde_json::to_string_pretty(value).unwrap_or_else(|_| "(object)".to_string()),
        serde_json::Value::Array(arr) => format!("(array with {} items)", arr.len()),
        _ => value.to_string(),
    }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
