/// Layout manager for the TUI application
use crate::actions::TabId;
use crate::stores::{AppLogsStore, LogsStore, UIStore, WorkflowsStore};
use crate::ui::{AdminPanel, MainPanel, RunInfoPanel, Sidebar};
use chrono;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Widget},
};

use crate::stores::LogViewerStore;
use crate::ui::LogViewerModal;

/// Render the complete application layout
pub fn render_layout(
    frame: &mut Frame,
    workflows_store: &WorkflowsStore,
    ui_store: &UIStore,
    _logs_store: &LogsStore,
    app_logs_store: &AppLogsStore,
    log_viewer_store: &LogViewerStore,
) {
    let area = frame.area();

    // Main layout: Header | (Tabs + Content) | Footer
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Tabs + Content
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Render header
    render_header(frame, vertical_chunks[0], workflows_store, ui_store);

    // Render tabs and content based on active tab
    let ui_state = ui_store.get_state();
    match ui_state.active_tab {
        TabId::Workflows => {
            render_workflows_with_tabs(frame, vertical_chunks[1], workflows_store, ui_store);
        }
        TabId::Admin => {
            render_admin_content(frame, vertical_chunks[1], app_logs_store);
        }
    }

    // Render footer
    render_footer(frame, vertical_chunks[2], ui_store);

    // Render log viewer modal on top if open
    let log_viewer_state = log_viewer_store.get_state();
    if log_viewer_state.is_open {
        let mut modal = LogViewerModal::new(log_viewer_state, log_viewer_store);
        modal.render(area, frame.buffer_mut());
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, active_tab: &TabId) {
    let tab_titles = vec!["1: Workflows", "2: Admin"];
    let selected_index = match active_tab {
        TabId::Workflows => 0,
        TabId::Admin => 1,
    };

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .select(selected_index)
        .divider("|");

    frame.render_widget(tabs, area);
}

fn render_workflows_with_tabs(
    frame: &mut Frame,
    area: Rect,
    workflows_store: &WorkflowsStore,
    ui_store: &UIStore,
) {
    // Split horizontally: (Tabs + Left panels) | Right panel
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(65), // Left section (Sidebar + Main Panel)
            Constraint::Percentage(35), // Right section (Recent Workflow Runs)
        ])
        .split(area);

    // Split left section vertically: Tabs | Content
    let left_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Content (Sidebar + Main Panel)
        ])
        .split(horizontal_chunks[0]);

    // Render tabs in left section only
    let ui_state = ui_store.get_state();
    render_tabs(frame, left_vertical[0], &ui_state.active_tab);

    // Split the left content area into Sidebar and Main Panel
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(31), // Sidebar (20% of total = ~31% of 65%)
            Constraint::Percentage(69), // Main Panel (45% of total = ~69% of 65%)
        ])
        .split(left_vertical[1]);

    // Render panels
    let workflows_state = workflows_store.get_state();
    let selected_index = workflows_store.get_selected_index();
    let selected_workflow = workflows_store.get_selected_workflow();

    let sidebar = Sidebar::from_state(&workflows_state, &ui_state, selected_index);
    sidebar.render(content_chunks[0], frame.buffer_mut());

    let main_panel = MainPanel::new(
        selected_workflow.clone(),
        &ui_state,
        workflows_state.main_panel_scroll_offset,
    );
    main_panel.render(content_chunks[1], frame.buffer_mut());

    // Render right panel (Recent Workflow Runs) - spans full height
    let run_info_panel = RunInfoPanel::new(
        workflows_state.recent_statuses.clone(),
        &ui_state,
        workflows_state.run_info_filter.clone(),
        workflows_state.run_info_scroll_offset,
    );
    run_info_panel.render(horizontal_chunks[1], frame.buffer_mut());
}

fn render_admin_content(frame: &mut Frame, area: Rect, app_logs_store: &AppLogsStore) {
    let app_logs_state = app_logs_store.get_state();
    let admin_panel = AdminPanel::from_state(&app_logs_state);
    admin_panel.render(area, frame.buffer_mut());
}

fn render_header(
    frame: &mut Frame,
    area: Rect,
    workflows_store: &WorkflowsStore,
    ui_store: &UIStore,
) {
    let workflows_state = workflows_store.get_state();
    let ui_state = ui_store.get_state();

    let status_string;
    let status = if workflows_state.is_loading {
        "Loading..."
    } else if let Some(err) = &workflows_state.error {
        err.as_str()
    } else if !ui_state.principal_online {
        if let Some(disconnect_ts) = ui_state.disconnect_since {
            let elapsed = chrono::Utc::now().timestamp() - disconnect_ts;
            status_string = format!("Disconnected ({}s)", elapsed);
            &status_string
        } else {
            "Disconnected"
        }
    } else {
        "Connected"
    };

    let status_color = if workflows_state.error.is_some() {
        Color::Red
    } else if workflows_state.is_loading {
        Color::Yellow
    } else if !ui_state.principal_online {
        Color::Red
    } else {
        Color::Green
    };

    let header_text = Line::from(vec![
        Span::styled(
            " CDKTR ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | Principal Status: "),
        Span::styled(status, Style::default().fg(status_color)),
        Span::raw(" | "),
    ]);

    Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL))
        .render(area, frame.buffer_mut());
}

fn render_footer(frame: &mut Frame, area: Rect, ui_store: &UIStore) {
    let ui_state = ui_store.get_state();

    let help_text = if ui_state.show_help {
        "Press ? to hide help"
    } else {
        match ui_state.active_tab {
            TabId::Workflows => {
                "q:Quit | 1/2:Switch Tab | j/k:Navigate | h/l:Panel | r:Refresh | ?:Help"
            }
            TabId::Admin => "q:Quit | 1/2:Switch Tab | j/k:Scroll | ?:Help",
        }
    };

    let footer_text = Line::from(vec![Span::raw(" "), Span::raw(help_text)]);

    Paragraph::new(footer_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL))
        .render(area, frame.buffer_mut());
}
