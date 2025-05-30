use std::collections::VecDeque;

pub mod data_structures;

/// helper function to convert a pipe delimited string
/// into a vecdeque of string tokens. Uses \ as the
/// defaul escape character for pipes
pub fn arg_str_to_vecd(s: &String) -> VecDeque<String> {
    let mut vecd = VecDeque::new();
    let mut fb = false;
    let mut current_arg = String::new();
    for ch in s.chars() {
        if fb {
            if ch == '|' {
                current_arg.push(ch);
                fb = false;
                continue;
            } else {
                current_arg.push('\\');
                fb = false;
                continue;
            }
        } else {
            if ch == '\\' {
                fb = true;
                continue;
            } else if ch == '|' {
                // pipe with no escape so end of arg
                vecd.push_back(current_arg);
                current_arg = String::new();
            } else {
                // normal character so push
                current_arg.push(ch);
            }
        }
    }
    if fb {
        // ended on a backslash so add to final arg
        current_arg.push('\\');
    };
    vecd.push_back(current_arg);
    vecd
}

/// similar helper function to arg_str_to_vecd to do the inverse and
/// encode a series of string arguments as a pipe-delimited string
/// adding escape \ where necessary
pub fn vecd_to_arg_str(vecd: &VecDeque<String>) -> String {
    let mut s = String::new();
    for arg in vecd {
        for ch in arg.chars() {
            if ch == '|' || ch == '\\' {
                // escape | and \ by adding another \
                s.push('\\');
            };
            s.push(ch);
        }
        s.push('|');
    }
    s.pop(); // remove final pipe
    s
}

pub fn get_instance_id() -> String {
    format!("{}@{}", whoami::username(), whoami::devicename(),)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arg_to_vecd() {
        let args = "hello|world".to_string();
        assert_eq!(
            arg_str_to_vecd(&args),
            vec!["hello".to_string(), "world".to_string()]
        )
    }

    #[test]
    fn test_arg_to_vecd_escape_pipe() {
        let args = r#"he\|\|o|world"#.to_string();
        assert_eq!(
            arg_str_to_vecd(&args),
            vec!["he||o".to_string(), "world".to_string()]
        )
    }

    #[test]
    fn test_arg_to_vecd_escape_final_backslash() {
        // backslashes should also be escaped if we want \ literal
        let args = r#"some\\path\\|file.rs"#.to_string();
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
        assert_eq!(vecd_to_arg_str(&args), "hello|world".to_string())
    }

    #[test]
    fn test_vecd_to_arg_str_escape_pipe() {
        let args: VecDeque<String> = vec!["he||o".to_string(), "world".to_string()].into();
        assert_eq!(vecd_to_arg_str(&args), r#"he\|\|o|world"#.to_string())
    }

    #[test]
    fn test_vecd_to_arg_str_escape_final_backslash() {
        // backslashes should also be escaped if we want \ literal
        let args: VecDeque<String> =
            vec![r#"some\path\"#.to_string(), "file.rs".to_string()].into();
        assert_eq!(
            vecd_to_arg_str(&args),
            r#"some\\path\\|file.rs"#.to_string()
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
            r#"python|-c|'import time;time.sleep(1);print("Done")'"#.to_string()
        )
    }

    #[test]
    fn test_arg_to_vecd_escape_outer_single_quote() {
        let args = r#"python|-c|'import time;time.sleep(1);print("Done")'"#.to_string();
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
