

#[derive(Debug, PartialEq, Clone)]
pub struct ProcessTask {
    pub command: String,
    pub args: Option<Vec<String>>
}