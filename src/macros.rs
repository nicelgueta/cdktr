
macro_rules! pipe_to_vec {
    ($st:expr) => {
        arg_str.split("|").mapcollect()
    }
}

pub(crate) use pipe_to_vec;