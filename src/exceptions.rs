#[derive(Debug, PartialEq)]
    pub enum ZMQParseError {
        ParseError(String),
        InvalidMessageType,
        InvalidTaskType
    }
    impl ZMQParseError {
        pub fn to_string(&self) -> String {
            match self {
                Self::ParseError(msg) => format!("ParseError: {msg}"),
                Self::InvalidMessageType => String::from("Invalid message type"),
                Self::InvalidTaskType => String::from("Invalid task type")
            }
        }
    }