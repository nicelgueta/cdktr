use std::{collections::VecDeque, env, time::Duration};

use crate::{exceptions::GenericError, macros::internal_get_cdktr_setting, zmq_helpers::{format_zmq_msg_str, get_server_tcp_uri}, ZMQ_MESSAGE_DELIMITER};
use log::warn;
pub mod data_structures;

/// helper function to convert a SOH delimited string
/// into a vecdeque of string tokens. No escape characters
/// are used for SOH delimited strings so any messages containing
/// SOH as values will be invalid
pub fn arg_str_to_vecd(s: &String) -> VecDeque<String> {
    s.split(ZMQ_MESSAGE_DELIMITER as char)
        .map(|s| s.to_string())
        .collect::<VecDeque<String>>()
}

/// similar helper function to arg_str_to_vecd to do the inverse and
/// encode a series of string arguments as a pipe-delimited string
/// adding escape \ where necessary
pub fn vecd_to_arg_str(vecd: &VecDeque<String>) -> String {
    format_zmq_msg_str(vecd.iter().map(|v| v.as_str()).collect::<Vec<&str>>())
}

pub fn get_instance_id() -> String {
    format!("{}@{}", whoami::username(), whoami::devicename(),)
}

pub fn str_or_blank<T: ToString>(s: Option<T>) -> String {
    match s {
        Some(t) => t.to_string(),
        None => "".to_string(),
    }
}

pub fn get_principal_uri() -> String {
    get_server_tcp_uri(
        &internal_get_cdktr_setting!(CDKTR_PRINCIPAL_HOST),
        internal_get_cdktr_setting!(CDKTR_PRINCIPAL_PORT, usize)
    )
}

pub fn get_default_timeout() -> Duration {
    Duration::from_millis(
        internal_get_cdktr_setting!(CDKTR_DEFAULT_TIMEOUT_MS, usize) as u64
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arg_to_vecd() {
        let args = format!("hello{}world", ZMQ_MESSAGE_DELIMITER as char);
        assert_eq!(
            arg_str_to_vecd(&args),
            vec!["hello".to_string(), "world".to_string()]
        )
    }

    #[test]
    fn test_arg_to_vecd_backslash() {
        // backslashes should also be escaped if we want \ literal
        let args = format!(r#"some\path\{}file.rs"#, ZMQ_MESSAGE_DELIMITER as char);
        assert_eq!(
            arg_str_to_vecd(&args),
            vec![r#"some\path\"#.to_string(), "file.rs".to_string()]
        )
    }

    #[test]
    fn test_arg_to_vecd_one_token() {
        let args = "helloworld".to_string();
        assert_eq!(arg_str_to_vecd(&args), vec!["helloworld".to_string()])
    }

    #[test]
    fn test_vecd_to_arg_str() {
        let args: VecDeque<String> = vec!["hello".to_string(), "world".to_string()].into();
        assert_eq!(
            vecd_to_arg_str(&args),
            format!("hello{}world", ZMQ_MESSAGE_DELIMITER as char)
        )
    }

    #[test]
    fn test_vecd_to_arg_str_backslash() {
        // backslashes should also be escaped if we want \ literal
        let args: VecDeque<String> =
            vec![r#"some\path\"#.to_string(), "file.rs".to_string()].into();
        assert_eq!(
            vecd_to_arg_str(&args),
            format!("some\\path\\{}file.rs", ZMQ_MESSAGE_DELIMITER as char)
        )
    }

    #[test]
    fn test_vecd_to_arg_str_one_token() {
        let args: VecDeque<String> = vec!["world".to_string()].into();
        assert_eq!(vecd_to_arg_str(&args), "world".to_string())
    }

    #[test]
    fn test_vecd_to_arg_str_escape_outer_single_quote() {
        let args: VecDeque<String> = vec![
            "python".to_string(),
            "-c".to_string(),
            r#"'import time;time.sleep(1);print("Done")'"#.to_string(),
        ]
        .into();
        assert_eq!(
            vecd_to_arg_str(&args),
            format!(
                r#"python{}-c{}'import time;time.sleep(1);print("Done")'"#,
                ZMQ_MESSAGE_DELIMITER as char, ZMQ_MESSAGE_DELIMITER as char
            )
        )
    }

    #[test]
    fn test_arg_to_vecd_escape_outer_single_quote() {
        let args = format!(
            r#"python{}-c{}'import time;time.sleep(1);print("Done")'"#,
            ZMQ_MESSAGE_DELIMITER as char, ZMQ_MESSAGE_DELIMITER as char
        );
        assert_eq!(
            arg_str_to_vecd(&args),
            vec![
                "python".to_string(),
                "-c".to_string(),
                r#"'import time;time.sleep(1);print("Done")'"#.to_string()
            ]
        )
    }
}
