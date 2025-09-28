#[macro_export]
macro_rules! get_cdktr_setting {
    ($setting:ident) => {
        ::std::env::var(stringify!($setting)).unwrap_or(cdktr_core::config::$setting.to_string())
    };
    ($setting:ident, usize) => {
        match ::std::env::var(stringify!($setting)) {
            Ok(v) => match v.parse() {
                Ok(i) => i,
                Err(e) => {
                    ::log::warn!(
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

macro_rules! internal_get_cdktr_setting {
    ($setting:ident) => {
        env::var(stringify!($setting)).unwrap_or(crate::config::$setting.to_string())
    };
    ($setting:ident, usize) => {
        match ::std::env::var(stringify!($setting)) {
            Ok(v) => match v.parse() {
                Ok(i) => i,
                Err(_e) => {
                    warn!(
                        "Env var setting {}, is not a valid unsigned integer. Using default",
                        stringify!($setting)
                    );
                    crate::config::$setting
                }
            },
            Err(_e) => crate::config::$setting,
        }
    };
}
pub(crate) use internal_get_cdktr_setting;
