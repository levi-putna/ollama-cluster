use std::io::{stdout, Stdout};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ocluster_client::ManagementClient;
use ocluster_protocol::{
    ClusterStatusResponse, EventResponse, ModelSummary, NodeDetailResponse, NodeSummary,
    RequestSummary,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::runtime::Runtime;

use crate::ui;

/// Available dashboard screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Overview,
    Nodes,
    NodeDetail,
    Models,
    Requests,
    Events,
    Config,
    Help,
}

impl Screen {
    /// All navigable screens in tab order.
    pub fn all() -> &'static [Screen] {
        &[
            Screen::Overview,
            Screen::Nodes,
            Screen::Models,
            Screen::Requests,
            Screen::Events,
            Screen::Config,
            Screen::Help,
        ]
    }

    /// Human-readable tab label.
    pub fn label(self) -> &'static str {
        match self {
            Screen::Overview => "Overview",
            Screen::Nodes => "Nodes",
            Screen::NodeDetail => "Node",
            Screen::Models => "Models",
            Screen::Requests => "Requests",
            Screen::Events => "Events",
            Screen::Config => "Config",
            Screen::Help => "Help",
        }
    }
}

/// Pending destructive action awaiting confirmation.
#[derive(Debug, Clone)]
pub enum PendingAction {
    DisableNode { name: String },
    DrainNode { name: String },
}

/// Snapshot of management API data shown in the dashboard.
#[derive(Debug, Clone, Default)]
pub struct DashboardData {
    pub status: Option<ClusterStatusResponse>,
    pub nodes: Vec<NodeSummary>,
    pub models: Vec<ModelSummary>,
    pub requests: Vec<RequestSummary>,
    pub events: Vec<EventResponse>,
    pub config: Option<serde_json::Value>,
    pub node_detail: Option<NodeDetailResponse>,
    pub last_error: Option<String>,
}

/// Dashboard application state.
pub struct App {
    pub screen: Screen,
    pub data: DashboardData,
    pub selected_index: usize,
    pub status_message: String,
    pub pending_action: Option<PendingAction>,
    pub quit: bool,
    pub force_refresh: bool,
    last_refresh: Instant,
}

impl App {
    /// Create a new dashboard application.
    pub fn new() -> Self {
        Self {
            screen: Screen::Overview,
            data: DashboardData::default(),
            selected_index: 0,
            status_message: String::from("Connecting…"),
            pending_action: None,
            quit: false,
            force_refresh: true,
            last_refresh: Instant::now() - Duration::from_secs(10),
        }
    }

    /// Whether data should be refreshed from the management API.
    pub fn should_refresh(&self) -> bool {
        self.force_refresh || self.last_refresh.elapsed() >= Duration::from_secs(2)
    }

    /// Mark refresh complete.
    pub fn refreshed(&mut self) {
        self.force_refresh = false;
        self.last_refresh = Instant::now();
    }

    /// Move tab selection forward or backward.
    pub fn next_screen(&mut self, forward: bool) {
        let screens = Screen::all();
        let current = screens.iter().position(|s| *s == self.screen).unwrap_or(0);
        let next = if forward {
            (current + 1) % screens.len()
        } else {
            (current + screens.len() - 1) % screens.len()
        };
        self.screen = screens[next];
        self.selected_index = 0;
        if self.screen != Screen::NodeDetail {
            self.data.node_detail = None;
        }
    }

    /// Clamp list selection after data changes.
    pub fn clamp_selection(&mut self) {
        let len = match self.screen {
            Screen::Nodes | Screen::NodeDetail => self.data.nodes.len(),
            Screen::Models => self.data.models.len(),
            Screen::Requests => self.data.requests.len(),
            Screen::Events => self.data.events.len(),
            _ => 0,
        };
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    /// Handle keyboard input.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }

        if self.pending_action.is_some() {
            self.handle_confirm_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') | KeyCode::F(1) => self.screen = Screen::Help,
            KeyCode::Tab => self.next_screen(true),
            KeyCode::BackTab => self.next_screen(false),
            KeyCode::Char('r') => self.force_refresh = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index = self.selected_index.saturating_add(1);
                self.clamp_selection();
            }
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Char('d') => self.queue_action(PendingAction::DisableNode {
                name: self.selected_node_name(),
            }),
            KeyCode::Char('D') => self.queue_action(PendingAction::DrainNode {
                name: self.selected_node_name(),
            }),
            _ => {}
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {}
            KeyCode::Char('n') | KeyCode::Esc => {
                self.status_message = String::from("Action cancelled");
                self.pending_action = None;
            }
            _ => {}
        }
    }

    fn handle_enter(&mut self) {
        if self.screen == Screen::Nodes && !self.data.nodes.is_empty() {
            self.screen = Screen::NodeDetail;
            self.force_refresh = true;
        }
    }

    fn selected_node_name(&self) -> String {
        self.data
            .nodes
            .get(self.selected_index)
            .map(|n| n.name.clone())
            .unwrap_or_default()
    }

    fn queue_action(&mut self, action: PendingAction) {
        let empty = match &action {
            PendingAction::DisableNode { name } | PendingAction::DrainNode { name } => {
                name.is_empty()
            }
        };
        if empty {
            return;
        }
        self.pending_action = Some(action);
    }
}

/// Run the interactive dashboard against the management API endpoint.
pub fn run_dashboard(endpoint: &str) -> Result<()> {
    let rt = Runtime::new().context("failed to start async runtime for dashboard")?;
    let client = ManagementClient::new(endpoint).context("invalid management endpoint")?;

    enable_raw_mode().context("failed to enable raw mode")?;
    stdout()
        .execute(EnterAlternateScreen)
        .context("failed to enter alternate screen")?;
    let mut terminal =
        Terminal::new(CrosstermBackend::new(stdout())).context("failed to create terminal")?;

    let mut app = App::new();
    let result = run_loop(&mut terminal, &rt, &client, &mut app);

    disable_raw_mode().ok();
    stdout().execute(LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    rt: &Runtime,
    client: &ManagementClient,
    app: &mut App,
) -> Result<()> {
    loop {
        if app.should_refresh() {
            refresh_data(rt, client, app);
            app.refreshed();
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.screen == Screen::Nodes || app.screen == Screen::NodeDetail {
                    handle_node_action_keys(rt, client, app, key);
                } else {
                    app.handle_key(key);
                }
            }
        }

        if app.quit {
            break;
        }
    }
    Ok(())
}

fn handle_node_action_keys(rt: &Runtime, client: &ManagementClient, app: &mut App, key: KeyEvent) {
    if let Some(action) = app.pending_action.clone() {
        if key.code == KeyCode::Char('y') {
            app.pending_action = None;
            execute_action(rt, client, app, action);
            return;
        }
        if matches!(key.code, KeyCode::Char('n') | KeyCode::Esc) {
            app.status_message = String::from("Action cancelled");
            app.pending_action = None;
            return;
        }
    }

    if key.modifiers.is_empty() {
        match key.code {
            KeyCode::Char('E') => {
                let name = app.selected_node_name();
                if !name.is_empty() {
                    match rt.block_on(client.enable_node(&name)) {
                        Ok(resp) => app.status_message = resp.message,
                        Err(e) => app.status_message = format!("Enable failed: {e}"),
                    }
                    app.force_refresh = true;
                }
                return;
            }
            KeyCode::Char('p') => {
                let name = app.selected_node_name();
                if !name.is_empty() {
                    match rt.block_on(client.probe_node(&name)) {
                        Ok(_) => app.status_message = format!("Probe succeeded for {name}"),
                        Err(e) => app.status_message = format!("Probe failed: {e}"),
                    }
                    app.force_refresh = true;
                }
                return;
            }
            KeyCode::Char('s') => match rt.block_on(client.sync_models()) {
                Ok(resp) => app.status_message = resp.message,
                Err(e) => app.status_message = format!("Sync failed: {e}"),
            },
            _ => {}
        }
    }

    app.handle_key(key);
}

fn execute_action(rt: &Runtime, client: &ManagementClient, app: &mut App, action: PendingAction) {
    let result = match action {
        PendingAction::DisableNode { name } => rt.block_on(client.disable_node(&name)),
        PendingAction::DrainNode { name } => rt.block_on(client.drain_node(&name)),
    };

    app.status_message = match result {
        Ok(resp) => resp.message,
        Err(e) => format!("Action failed: {e}"),
    };
    app.force_refresh = true;
}

fn refresh_data(rt: &Runtime, client: &ManagementClient, app: &mut App) {
    let mut data = DashboardData::default();

    match rt.block_on(client.cluster_status()) {
        Ok(status) => data.status = Some(status),
        Err(e) => data.last_error = Some(e.to_string()),
    }

    if let Ok(nodes) = rt.block_on(client.list_nodes()) {
        data.nodes = nodes;
    }

    if let Ok(models) = rt.block_on(client.list_models()) {
        data.models = models;
    }

    if let Ok(requests) = rt.block_on(client.list_requests()) {
        data.requests = requests;
    }

    if let Ok(events) = rt.block_on(client.list_events()) {
        data.events = events;
    }

    if let Ok(config) = rt.block_on(client.show_config()) {
        data.config = Some(config);
    }

    if app.screen == Screen::NodeDetail {
        let name = app.selected_node_name();
        if !name.is_empty() {
            if let Ok(detail) = rt.block_on(client.get_node(&name)) {
                data.node_detail = Some(detail);
            }
        }
    }

    app.data = data;
    app.clamp_selection();

    if app.data.last_error.is_none() {
        app.status_message = String::from("Connected — auto-refresh every 2s");
    } else if let Some(err) = &app.data.last_error {
        app.status_message = format!("Error: {err}");
    }
}
