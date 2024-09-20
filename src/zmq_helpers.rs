use std::time::Duration;

use crate::{
    exceptions::{GenericError, ZMQParseError},
    models::ZMQArgs,
    utils::arg_str_to_vec,
};
use tokio::time::timeout;
use zeromq::{RepSocket, ReqSocket, Socket, SocketRecv, ZmqMessage};

pub static DEFAULT_TIMEOUT: Duration = Duration::from_millis(1000);

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

pub async fn get_req_timeout(
    host: &str,
    port: usize,
    duration: Duration,
) -> Result<ReqSocket, GenericError> {
    let uri = get_server_tcp_uri(host, port);
    let res = tokio::spawn(timeout(duration, async move { get_zmq_req(&uri).await }))
        .await
        .expect("Encountered join error");
    match res {
        Ok(req) => Ok(req),
        Err(_e) => Err(GenericError::TimeoutError),
    }
}

pub async fn get_zmq_rep(endpoint_uri: &str) -> RepSocket {
    let mut rep = RepSocket::new();
    rep.bind(endpoint_uri)
        .await
        .expect("Failed to connect to REQ socket");
    rep
}

pub fn get_server_tcp_uri(host: &str, port: usize) -> String {
    return format!("tcp://{host}:{port}");
}

/// calling .await on a ReqSocket.recv() could hang if the agent has died
/// so this function spawns the recv in a separate coroutine and
/// the calling process waits on a responds from the join handle. Given a certain
/// duration if no response is received it kills the spawned coroutine and
/// returns an error
pub async fn wait_on_recv(
    mut req: ReqSocket,
    duration: Duration,
) -> Result<ZmqMessage, GenericError> {
    let join_res = tokio::spawn(timeout(duration, async move { req.recv().await })).await;
    match join_res {
        Ok(time_r) => match time_r {
            Ok(zmq_r) => match zmq_r {
                Ok(msg) => Ok(msg),
                Err(e) => Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                    format!("ZMQ failure: {}", e.to_string()),
                ))),
            },
            Err(_e) => Err(GenericError::TimeoutError),
        },
        Err(e) => Err(GenericError::RuntimeError(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::{SocketRecv, SocketSend};

    async fn get_req(host: &str, port: usize,) -> Result<ReqSocket, GenericError> {
        get_req_timeout(host, port, Duration::from_millis(500)).await
    }

    #[tokio::test]
    async fn test_get_req_ok() {
        let host = String::from("0.0.0.0");
        let port = 9999;
        let endpoint = get_server_tcp_uri(&host, port);
        let mut rep = get_zmq_rep(&endpoint).await;
        tokio::spawn(async move {
            rep.recv().await.unwrap();
            rep.send("OK".into()).await.unwrap()
        });
        assert!(get_req(&host, port).await.is_ok());
    }

    #[tokio::test]
    async fn test_get_req_timeout() {
        let host = String::from("0.0.0.0");
        let port = 9998;
        assert!(get_req(&host, port).await.is_err());
    }

    #[tokio::test]
    async fn test_wait_on_recv_ok() {
        let host = String::from("0.0.0.0");
        let port = 9997;
        let endpoint = get_server_tcp_uri(&host, port);
        let mut rep = get_zmq_rep(&endpoint).await;
        tokio::spawn(async move {
            rep.recv().await.unwrap();
            rep.send("OK".into()).await.unwrap()
        });
        let mut req = get_zmq_req(&endpoint).await;
        req.send("hey".into()).await.unwrap();
        let dur = Duration::from_millis(500);
        let res = wait_on_recv(req, dur).await;
        assert!(res.is_ok())
    }
}
