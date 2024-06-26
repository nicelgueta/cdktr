
pub fn arg_str_to_vec(s: &str) -> Vec<String> {
    s.split("|").map(|x|x.to_string()).collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arg_to_vec() {
        let args = "hello|world";
        assert_eq!(arg_str_to_vec(args), vec![
            "hello".to_string(), "world".to_string()
        ])
    }

    #[test]
    fn test_arg_to_vec_empty() {
        let args = "helloworld";
        assert_eq!(arg_str_to_vec(args), vec![
            "helloworld".to_string()
        ])
    }
}