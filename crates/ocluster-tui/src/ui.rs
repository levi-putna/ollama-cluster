use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap};
use ratatui::Frame;

use crate::app::{App, PendingAction, Screen};

/// Render the full dashboard frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_tabs(frame, app, chunks[0]);
    draw_content(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);
}

fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Screen::all()
        .iter()
        .map(|screen| {
            let label = screen.label();
            if *screen == app.screen {
                Line::from(Span::styled(
                    format!(" {label} "),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(format!(" {label} "))
            }
        })
        .collect();

    let selected = Screen::all()
        .iter()
        .position(|s| *s == app.screen)
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ocluster dashboard "),
        )
        .select(selected)
        .divider("|");

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.screen {
        Screen::Overview => draw_overview(frame, app, area),
        Screen::Nodes => draw_nodes(frame, app, area),
        Screen::NodeDetail => draw_node_detail(frame, app, area),
        Screen::Models => draw_models(frame, app, area),
        Screen::Requests => draw_requests(frame, app, area),
        Screen::Events => draw_events(frame, app, area),
        Screen::Config => draw_config(frame, app, area),
        Screen::Help => draw_help(frame, area),
    }

    if let Some(action) = &app.pending_action {
        draw_confirm_modal(frame, app, area, action);
    }
}

fn draw_overview(frame: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(status) = &app.data.status {
        format!(
            "Cluster state: {}\n\
             Uptime: {}s\n\n\
             Nodes: {} total, {} ready, {} unavailable, {} draining\n\
             Models: {}\n\
             Active requests: {}, queued: {}",
            status.state,
            status.uptime_seconds,
            status.nodes_total,
            status.nodes_ready,
            status.nodes_unavailable,
            status.nodes_draining,
            status.models_total,
            status.active_requests,
            status.queued_requests,
        )
    } else {
        String::from("No cluster status available.\nEnsure `ocluster serve` is running.")
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Cluster overview "),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn draw_nodes(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["Name", "Admin", "Runtime", "URL", "Active", "Models"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .data
        .nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let style = if idx == app.selected_index {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(node.name.clone()),
                Cell::from(format!("{:?}", node.admin_state)),
                Cell::from(format!("{:?}", node.runtime_state)),
                Cell::from(node.url.clone()),
                Cell::from(node.active_requests.to_string()),
                Cell::from(node.model_count.to_string()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Nodes — Enter detail | E enable | d disable | D drain | p probe | s sync "),
    );

    frame.render_widget(table, area);
}

fn draw_node_detail(frame: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(detail) = &app.data.node_detail {
        format!(
            "Name: {}\nURL: {}\nAdmin: {:?}\nRuntime: {:?}\nVersion: {}\n\
             Max concurrent: {}\nActive: {}\nModels: {}\n\nModel list:\n  {}",
            detail.summary.name,
            detail.summary.url,
            detail.summary.admin_state,
            detail.summary.runtime_state,
            detail.summary.ollama_version.as_deref().unwrap_or("-"),
            detail.max_concurrent,
            detail.summary.active_requests,
            detail.models.len(),
            detail.models.join("\n  "),
        )
    } else {
        String::from("Select a node on the Nodes tab and press Enter.")
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Node detail — Esc/Tab back "),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn draw_models(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["Model", "Nodes", "Ready", "Loaded"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .data
        .models
        .iter()
        .enumerate()
        .map(|(idx, model)| {
            let style = if idx == app.selected_index {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(model.name.clone()),
                Cell::from(model.node_count.to_string()),
                Cell::from(model.ready_nodes.to_string()),
                Cell::from(model.loaded_instances.to_string()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(30),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Models "));

    frame.render_widget(table, area);
}

fn draw_requests(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["ID", "Model", "Node", "Duration ms", "Streaming"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .data
        .requests
        .iter()
        .enumerate()
        .map(|(idx, req)| {
            let style = if idx == app.selected_index {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(truncate(&req.id, 12)),
                Cell::from(req.model.clone()),
                Cell::from(req.node.clone()),
                Cell::from(req.duration_ms.to_string()),
                Cell::from(req.streaming.to_string()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Min(20),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Active requests "),
    );

    frame.render_widget(table, area);
}

fn draw_events(frame: &mut Frame, app: &App, area: Rect) {
    let lines: Vec<Line> = app
        .data
        .events
        .iter()
        .map(|event| {
            Line::from(format!(
                "{} [{}] {} — {}",
                event.created_at,
                event.event_type,
                event.target.as_deref().unwrap_or("-"),
                event.message,
            ))
        })
        .collect();

    let paragraph = Paragraph::new(if lines.is_empty() {
        vec![Line::from("No events recorded yet.")]
    } else {
        lines
    })
    .block(Block::default().borders(Borders::ALL).title(" Events "))
    .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn draw_config(frame: &mut Frame, app: &App, area: Rect) {
    let text = app
        .data
        .config
        .as_ref()
        .map(|c| serde_json::to_string_pretty(c).unwrap_or_else(|_| c.to_string()))
        .unwrap_or_else(|| String::from("Configuration unavailable"));

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Configuration summary "),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let help = "\
Navigation\n\
  Tab / Shift+Tab   Switch views\n\
  j / k, ↑ / ↓      Move selection\n\
  Enter             Open node detail (Nodes view)\n\
  r                 Refresh now\n\
  q, Ctrl+c         Quit\n\
\n\
Node actions (Nodes / Node detail)\n\
  E                 Enable selected node\n\
  d                 Disable selected node (confirm)\n\
  D                 Drain selected node (confirm)\n\
  p                 Probe selected node\n\
  s                 Sync models across cluster\n\
\n\
Data refreshes automatically every 2 seconds via the management API.";

    let paragraph = Paragraph::new(help)
        .block(Block::default().borders(Borders::ALL).title(" Help "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let paragraph = Paragraph::new(app.status_message.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, area);
}

fn draw_confirm_modal(frame: &mut Frame, _app: &App, area: Rect, action: &PendingAction) {
    let message = match action {
        PendingAction::DisableNode { name } => format!("Disable node '{name}'?  y/n"),
        PendingAction::DrainNode { name } => format!("Drain node '{name}'?  y/n"),
    };

    let popup_area = centred_rect(60, 20, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    let paragraph = Paragraph::new(message)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm action ")
                .style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}

fn centred_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate(value: &str, max: usize) -> String {
    if value.len() <= max {
        value.to_string()
    } else {
        format!("{}…", &value[..max.saturating_sub(1)])
    }
}
