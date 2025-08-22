use std::time::Duration;

use crate::exceptions::{GenericError, ZMQParseError};
use tokio::time::timeout;
use zeromq::{
    PubSocket, PullSocket, PushSocket, RepSocket, ReqSocket, Socket, SocketRecv, SocketSend,
    SubSocket, ZmqMessage,
};

pub async fn get_zmq_req(endpoint_uri: &str) -> Result<ReqSocket, GenericError> {
    let mut req = ReqSocket::new();
    req.connect(endpoint_uri)
        .await
        .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
    Ok(req)
}

pub async fn get_zmq_rep(endpoint_uri: &str) -> Result<RepSocket, GenericError> {
    let mut rep = RepSocket::new();
    rep.bind(endpoint_uri)
        .await
        .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
    Ok(rep)
}

pub async fn get_zmq_pub(endpoint_uri: &str) -> Result<PubSocket, GenericError> {
    let mut pub_socket = PubSocket::new();
    pub_socket
        .bind(endpoint_uri)
        .await
        .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
    Ok(pub_socket)
}

pub async fn get_zmq_sub(endpoint_uri: &str, topic: &str) -> Result<SubSocket, GenericError> {
    let mut sub_socket = SubSocket::new();
    sub_socket
        .connect(endpoint_uri)
        .await
        .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
    sub_socket
        .subscribe(topic)
        .await
        .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
    Ok(sub_socket)
}

pub async fn get_zmq_pull(endpoint_uri: &str) -> Result<PullSocket, GenericError> {
    let mut pull_socket = PullSocket::new();
    pull_socket
        .bind(endpoint_uri)
        .await
        .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
    Ok(pull_socket)
}

pub async fn get_zmq_push(endpoint_uri: &str) -> Result<PushSocket, GenericError> {
    let mut push_socket = PushSocket::new();
    push_socket
        .connect(endpoint_uri)
        .await
        .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
    Ok(push_socket)
}

pub fn get_server_tcp_uri(host: &str, port: usize) -> String {
    return format!("tcp://{host}:{port}");
}

/// calling .await on a ReqSocket.recv() or ReqSocket.send() could hang if the message receiver has died
/// so this function spawns the recv in a separate coroutine and
/// the calling process waits on a responds from the join handle. Given a certain
/// duration if no response is received it kills the spawned coroutine and
/// returns an error
pub async fn send_recv_with_timeout(
    tcp_uri: String,
    zmq_msg: ZmqMessage,
    duration: Duration,
) -> Result<ZmqMessage, GenericError> {
    // spawn the timeout coroutine
    let join_res = tokio::spawn(timeout(duration, async move {
        let mut req = get_zmq_req(&tcp_uri).await?;
        let send_res = req.send(zmq_msg).await;
        match send_res {
            Ok(_) => {
                let recv_res = req.recv().await;
                match recv_res {
                    Ok(zmq_msg) => Ok(zmq_msg),
                    Err(e) => Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                        e.to_string(),
                    ))),
                }
            }
            Err(e) => Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                e.to_string(),
            ))),
        }
    }))
    .await;

    // handle the outcome
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
    use tokio::time::sleep;
    use zeromq::{SocketRecv, SocketSend};

    async fn get_req_timeout(
        host: &str,
        port: usize,
        duration: Duration,
    ) -> Result<ReqSocket, GenericError> {
        let uri = get_server_tcp_uri(host, port);
        let res = tokio::spawn(timeout(duration, async move {
            get_zmq_req(&uri).await.unwrap()
        }))
        .await
        .expect("Encountered join error");
        match res {
            Ok(req) => Ok(req),
            Err(_e) => Err(GenericError::TimeoutError),
        }
    }

    async fn get_req(host: &str, port: usize) -> Result<ReqSocket, GenericError> {
        get_req_timeout(host, port, Duration::from_millis(500)).await
    }

    #[tokio::test]
    async fn test_get_req_ok() {
        let host = String::from("0.0.0.0");
        let port = 9999;
        let endpoint = get_server_tcp_uri(&host, port);
        let mut rep = get_zmq_rep(&endpoint).await.unwrap();
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
    async fn test_send_recv_with_timeout_good() {
        let host = String::from("0.0.0.0");
        let port = 9997;
        let endpoint = get_server_tcp_uri(&host, port);
        let mut rep = get_zmq_rep(&endpoint).await.unwrap();
        tokio::spawn(async move {
            rep.recv().await.unwrap();
            rep.send("OK".into()).await.unwrap()
        });
        assert!(
            send_recv_with_timeout(endpoint, ZmqMessage::from("hello"), Duration::from_secs(1))
                .await
                .is_ok()
        )
    }

    #[tokio::test]
    async fn test_send_recv_with_timeout_times_out() {
        let host = String::from("0.0.0.0");
        let port = 9996;
        let endpoint = get_server_tcp_uri(&host, port);
        let mut rep = get_zmq_rep(&endpoint).await.unwrap();
        tokio::spawn(async move {
            rep.recv().await.unwrap();
            sleep(Duration::from_millis(500)).await;
            rep.send("OK".into()).await.unwrap()
        });
        assert!(send_recv_with_timeout(
            endpoint,
            ZmqMessage::from("hello"),
            Duration::from_millis(1)
        )
        .await
        .is_err())
    }

    #[test]
    fn test_get_agent_tcp_uri() {
        let host = "localhost";
        let port = 1234 as usize;
        assert_eq!(get_server_tcp_uri(host, port), "tcp://localhost:1234")
    }
}
