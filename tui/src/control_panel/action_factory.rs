pub const ACTIONS: [&'static str; 2] = ["Ping", "List Tasks"];

#[derive(Debug, Clone)]
struct Ping;

#[derive(Debug, Clone)]
struct ListTasks;

#[derive(Debug, Clone)]
pub enum ActionPane {
    Ping(Ping),
    ListTasks(ListTasks),
}

/// Factory function to create the action utility body for the 
/// action panel for each given action
pub fn factory(action: &str) -> ActionPane {
    ActionPane::Ping(Ping)
}