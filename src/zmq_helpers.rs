use zeromq::ZmqMessage;
use crate::{models::ZMQArgs, utils::arg_str_to_vec};
use crate::server::models::ClientResponseMessage;

impl Into<ZMQArgs> for ZmqMessage {
    fn into(self) -> ZMQArgs {
        let raw_msg = String::try_from(self);
        let raw_string = match raw_msg {
            Ok(s) => s,
            Err(e_str) => e_str.to_string()
        };
        let arg_vec = arg_str_to_vec(raw_string);
        ZMQArgs::from(arg_vec)
    }
}
