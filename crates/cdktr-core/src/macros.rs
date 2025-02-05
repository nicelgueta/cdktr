/// Takes first first argument of a ZMQArgs object whih should
/// be a json string and attempts to parse it into a model.
macro_rules! args_to_model {
    ($args:expr, $model:ident) => {
        if $args.len() == 0 {
            Err(RepReqError::ParseError(
                "Payload in arg index 0 is required for this command".to_string(),
            ))
        } else {
            let args_v: Vec<String> = $args.into();
            let parse_res: Result<$model, serde_json::Error> = serde_json::from_str(&args_v[0]);
            match parse_res {
                Ok(task) => Ok(task),
                Err(e) => Err(RepReqError::ParseError(format!(
                    "Invalid JSON for {}. Error: {}",
                    stringify!($model),
                    e.to_string()
                ))),
            }
        }
    };
}
