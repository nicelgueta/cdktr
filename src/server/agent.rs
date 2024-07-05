use async_trait::async_trait;
use super::{
    msg::AgentRequest, 
    models::{
        traits::Server,
        ClientResponseMessage
    }
};

pub struct AgentServer {
    /// ID of the publisher currently subscribed to
    publisher_id: String
}

impl AgentServer {
    pub fn new() -> Self {
    
        // start with an empty string - the first heartbeat from the principal 
        //will correct this to the new value
        Self {publisher_id: "".to_string()}
    }
}

#[async_trait]
impl Server<AgentRequest> for AgentServer {
    
    async fn handle_client_message(
        &self, 
        cli_msg: AgentRequest
    ) -> (ClientResponseMessage, bool) {
        match cli_msg {
            AgentRequest::Ping => (ClientResponseMessage::Pong, false),
            AgentRequest::Heartbeat => (ClientResponseMessage::Heartbeat(self.publisher_id.clone()), false),
            AgentRequest::Reconnect => (ClientResponseMessage::Success, true)

        }
    }
}
