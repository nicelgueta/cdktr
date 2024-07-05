use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use zeromq::{Socket, PubSocket};
use super::{
    msg::PrincipalRequest, 
    models::{
        traits::Server,
        ClientResponseMessage
    }
};

pub struct PrincipalServer {
    publisher: Arc<Mutex<PubSocket>>
}

impl PrincipalServer {
    pub fn new(publisher: Arc<Mutex<PubSocket>>) -> Self  {
        Self {publisher}
    }
}

#[async_trait]
impl Server<PrincipalRequest> for PrincipalServer {

    async fn handle_client_message(
        &self, 
        cli_msg: PrincipalRequest
    ) -> (ClientResponseMessage, bool) {
        match cli_msg {
            PrincipalRequest::Ping => (ClientResponseMessage::Pong, false),
        }
    }
}
