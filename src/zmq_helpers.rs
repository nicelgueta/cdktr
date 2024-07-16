use std::time::Duration;

use crate::{exceptions::{ZMQParseError, GenericError}, models::ZMQArgs, utils::arg_str_to_vec};
use zeromq::{RepSocket, ReqSocket, Socket, SocketRecv, ZmqMessage};
use tokio::time::timeout;
impl Into<ZMQArgs> for ZmqMessage {
    fn into(self) -> ZMQArgs {
        let raw_msg = String::try_from(self);
        let raw_string = match raw_msg {
            Ok(s) => s,
            Err(e_str) => e_str.to_string(),
        };
        let arg_vec = arg_str_to_vec(raw_string);
        ZMQArgs::from(arg_vec)
    }
}

/// 
pub async fn get_zmq_req(endpoint_uri: &str) -> ReqSocket {
    let mut req = ReqSocket::new();
    req.connect(endpoint_uri)
        .await
        .expect("Failed to connect to REQ socket");
    req
}

pub async fn get_req_timeout(remote_id: &String, duration: Duration) -> Result<ReqSocket, GenericError> {
    let uri = get_agent_tcp_uri(remote_id);
    let res = tokio::spawn(
        timeout(
            duration,
            async move {get_zmq_req(&uri).await}
        )
    ).await.expect("Encountered join error");
    match res {
        Ok(req) => Ok(req),
        Err(_e) => Err(GenericError::TimeoutError)
    }

}

pub async fn get_zmq_rep(endpoint_uri: &str) -> RepSocket {
    let mut rep = RepSocket::new();
    rep.bind(endpoint_uri)
        .await
        .expect("Failed to connect to REQ socket");
    rep
}

pub fn get_agent_tcp_uri(agent_id: &String) -> String {
    // TODO: Change to use datastore instead
    // since these Ids will change
    return format!("tcp://0.0.0.0:{}", agent_id);
}


/// calling .await on a ReqSocket.recv() could hang if the agent has died
/// so this function spawns the recv in a separate coroutine and 
/// the calling process waits on a responds from the join handle. Given a certain
/// duration if no response is received it kills the spawned coroutine and 
/// returns an error
pub async fn wait_on_recv(mut req: ReqSocket, duration: Duration) -> Result<ZmqMessage, GenericError> {
    let join_res = tokio::spawn (
        timeout(duration, async move {req.recv().await})
    ).await;
    match join_res {
        Ok(time_r) => match time_r {
            Ok(zmq_r) => match zmq_r {
                Ok(msg) => Ok(msg),
                Err(e) => Err(GenericError::ZMQParseError(
                    ZMQParseError::ParseError(
                        format!("ZMQ failure: {}", e.to_string())
                )))
            },
            Err(_e) => Err(GenericError::TimeoutError)
        },
        Err(e) => Err(GenericError::RuntimeError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::{SocketRecv, SocketSend};

    async fn get_req(agent_id: &String) -> Result<ReqSocket, GenericError> {
        get_req_timeout(
            agent_id,
            Duration::from_millis(500)
        ).await
    }

    #[tokio::test]
    async fn test_get_req_ok(){
        let agent_id = String::from("9999");
        let endpoint = get_agent_tcp_uri(&agent_id);
        let mut rep = get_zmq_rep(&endpoint).await;
        tokio::spawn(
            async move {
                rep.recv().await.unwrap();
                rep.send("OK".into()).await.unwrap()
            }
        );
        assert!(get_req(&agent_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_get_req_timeout(){
        let agent_id = String::from("9998");
        assert!(get_req(&agent_id).await.is_err());
    }

    #[tokio::test]
    async fn test_wait_on_recv_ok(){
        let agent_id = String::from("9997");
        let endpoint = get_agent_tcp_uri(&agent_id);
        let mut rep = get_zmq_rep(&endpoint).await;
        tokio::spawn(
            async move {
                rep.recv().await.unwrap();
                rep.send("OK".into()).await.unwrap()
            }
        );
        let mut req = get_zmq_req(
            &endpoint
        ).await;
        req.send("hey".into()).await.unwrap();
        let dur = Duration::from_millis(500);
        let res = wait_on_recv(req, dur).await;
        assert!(res.is_ok())
    }
}