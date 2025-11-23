/// DAG visualization component using box-drawing characters
use cdktr_workflow::{Task, WorkFlowDAG};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::{HashMap, HashSet, VecDeque};

/// Renders a DAG as text lines using box-drawing characters
pub fn render_dag(dag: &WorkFlowDAG) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Get first tasks (root nodes)
    let first_tasks = dag.get_first_tasks();

    if first_tasks.is_empty() {
        lines.push(Line::from(Span::styled(
            "No tasks in workflow",
            Style::default().fg(Color::DarkGray),
        )));
        return lines;
    }

    lines.push(Line::from(Span::styled(
        "Task Dependencies:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Build the tree structure
    let mut visited = HashSet::new();
    let mut queue: VecDeque<(String, usize, bool)> = VecDeque::new();

    // Start with root tasks
    for (idx, task_id) in first_tasks.iter().enumerate() {
        let is_last = idx == first_tasks.len() - 1;
        queue.push_back((task_id.clone(), 0, is_last));
    }

    while let Some((task_id, level, is_last)) = queue.pop_front() {
        if visited.contains(&task_id) {
            continue;
        }
        visited.insert(task_id.clone());

        // Get task info
        let task = dag.get_task(&task_id);
        let task_name = task.map(|t| t.name()).unwrap_or(&task_id);

        // Create indentation and tree characters
        let mut prefix = String::new();
        for _ in 0..level {
            prefix.push_str("    ");
        }

        let connector = if is_last { "└── " } else { "├── " };
        let line_text = format!("{}{}{} ({})", prefix, connector, task_name, task_id);

        lines.push(Line::from(vec![
            Span::styled(prefix.clone(), Style::default().fg(Color::DarkGray)),
            Span::styled(connector, Style::default().fg(Color::DarkGray)),
            Span::styled(task_name.to_string(), Style::default().fg(Color::Green)),
            Span::styled(
                format!(" ({})", task_id),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Get dependents (children)
        if let Ok(dependents) = dag.get_dependents(&task_id) {
            for (dep_idx, dependent_id) in dependents.iter().enumerate() {
                let is_last_child = dep_idx == dependents.len() - 1;
                queue.push_back((dependent_id.to_string(), level + 1, is_last_child));
            }
        }
    }

    lines
}

/// Renders task count summary
pub fn render_task_summary(dag: &WorkFlowDAG) -> Vec<Line<'static>> {
    let node_count = dag.node_count();
    let first_tasks = dag.get_first_tasks();

    vec![
        Line::from(vec![
            Span::styled("Total Tasks: ", Style::default().fg(Color::Yellow)),
            Span::raw(node_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Entry Points: ", Style::default().fg(Color::Yellow)),
            Span::raw(first_tasks.len().to_string()),
            Span::styled(" (", Style::default().fg(Color::DarkGray)),
            Span::styled(first_tasks.join(", "), Style::default().fg(Color::Green)),
            Span::styled(")", Style::default().fg(Color::DarkGray)),
        ]),
    ]
}
