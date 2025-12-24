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
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::io;

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

        Ok(Self {
            db,
            specs,
            list_state,
            selected_tab: 0,
            should_quit: false,
            filter_boundary: None,
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
            .split(f.size());

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

        let titles = vec!["Overview", "Requirements", "Tasks", "Decisions", "History"];
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
                    _ => {}
                }
            }
        } else {
            let empty = Paragraph::new("No spec selected")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, tabs_area[1]);
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
        let footer = Paragraph::new(
            "  ↑/↓: Navigate  Tab: Switch Tab  1-4: Filter Boundary  r: Refresh  q/Esc: Quit"
        )
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
        self.selected_tab = (self.selected_tab + 1) % 5;
    }

    /// Navigate to previous tab
    fn previous_tab(&mut self) {
        if self.selected_tab == 0 {
            self.selected_tab = 4;
        } else {
            self.selected_tab -= 1;
        }
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
}
