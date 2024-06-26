

#[derive(Debug,PartialEq)]
pub struct ProcessTask {
    pub command: String,
    pub args: Option<Vec<String>>
}