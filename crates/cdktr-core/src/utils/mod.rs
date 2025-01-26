use std::collections::VecDeque;

pub mod data_structures;

pub fn arg_str_to_vec(s: String) -> VecDeque<String> {
    s.split("|").map(|x| x.to_string()).collect()
}

pub fn get_instance_id(host: &str, port: usize) -> String {
    let mut id = String::new();
    id.push_str(host);
    id.push_str("-");
    let port_s = port.to_string();
    id.push_str(&port_s);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arg_to_vec() {
        let args = "hello|world".to_string();
        assert_eq!(
            arg_str_to_vec(args),
            vec!["hello".to_string(), "world".to_string()]
        )
    }

    #[test]
    fn test_arg_to_vec_empty() {
        let args = "helloworld".to_string();
        assert_eq!(arg_str_to_vec(args), vec!["helloworld".to_string()])
    }
}
