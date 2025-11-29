use serde::Serialize;

#[derive(clap::ValueEnum, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstanceType {
    PRINCIPAL,
    AGENT,
}
impl InstanceType {
    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        match self {
            Self::AGENT => String::from("AGENT"),
            Self::PRINCIPAL => String::from("PRINCIPAL"),
        }
    }
}

#[derive(clap::ValueEnum, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskAction {
    /// action to create a new task in the principal database
    Create,
    Trigger,
}
