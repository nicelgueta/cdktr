use super::models::TaskAction;

#[derive(clap::Args)]
#[command(version, about, long_about = None)]
pub struct TaskArgs {
    /// Create a new scheduled task
    #[arg(long, short)]
    action: TaskAction,
}
