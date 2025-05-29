/// Takes first first argument of a ZMQArgs object whih should
/// be a json string and attempts to parse it into a model.
macro_rules! args_to_model {
    ($args:expr, $model:ident) => {
        if $args.len() == 0 {
            Err(GenericError::ParseError(
                "Payload in arg index 0 is required for this command".to_string(),
            ))
        } else {
            let args_v: Vec<String> = $args.into();
            let parse_res: Result<$model, serde_json::Error> = serde_json::from_str(&args_v[0]);
            match parse_res {
                Ok(task) => Ok(task),
                Err(e) => Err(GenericError::ParseError(format!(
                    "Invalid JSON for {}. Error: {}",
                    stringify!($model),
                    e.to_string()
                ))),
            }
        }
    };
}

#[macro_export]
macro_rules! get_cdktr_setting {
    ($setting:ident) => {
        env::var(stringify!($setting)).unwrap_or(cdktr_core::config::$setting.to_string())
    };
    ($setting:ident, usize) => {
        match env::var(stringify!($setting)) {
            Ok(v) => match v.parse() {
                Ok(i) => i,
                Err(e) => {
                    warn!(
                        "Env var setting {}, is not a valid unsigned integer. Using default",
                        stringify!($setting)
                    );
                    cdktr_core::config::$setting
                }
            },
            Err(e) => cdktr_core::config::$setting,
        }
    };
}
