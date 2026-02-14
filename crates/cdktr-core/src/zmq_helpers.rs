use std::time::Duration;

use crate::{
    exceptions::{GenericError, ZMQParseError},
    macros,
};
use log::warn;
use tokio::time::timeout;
use zeromq::{
    PubSocket, PullSocket, PushSocket, RepSocket, ReqSocket, Socket, SocketRecv, SocketSend,
    SubSocket, ZmqMessage,
};

pub static ZMQ_MESSAGE_DELIMITER: u8 = b'\x01';

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
    let cnxn_timeout = macros::internal_get_cdktr_setting!(CDKTR_DEFAULT_ZMQ_TIMEOUT_MS, usize);
    let push_socket_res = timeout(Duration::from_millis(cnxn_timeout as u64), async {
        let mut push_socket = PushSocket::new();
        push_socket
            .connect(endpoint_uri)
            .await
            .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
        Ok(push_socket)
    })
    .await
    .map_err(|_e| GenericError::ZMQTimeoutError)?;
    push_socket_res
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
        let result = match send_res {
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
        };
        // Explicitly close the socket to release file descriptors
        let close_errors = req.close().await;
        if !close_errors.is_empty() {
            warn!("Errors closing ZMQ socket: {:?}", close_errors);
        }
        result
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
            Err(_e) => Err(GenericError::ZMQTimeoutError),
        },
        Err(e) => Err(GenericError::RuntimeError(e.to_string())),
    }
}

pub async fn push_with_timeout(
    push_socket: &mut PushSocket,
    duration: Duration,
    msg: ZmqMessage,
) -> Result<(), GenericError> {
    let push_res = timeout(duration, push_socket.send(msg)).await;
    match push_res {
        Ok(r) => match r {
            Ok(()) => Ok(()),
            Err(e) => Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                e.to_string(),
            ))),
        },
        Err(_e) => Err(GenericError::ZMQTimeoutError),
    }
}

pub async fn sub_with_timeout(
    sub_socket: &mut SubSocket,
    duration: Duration,
) -> Result<ZmqMessage, GenericError> {
    let push_res = timeout(duration, sub_socket.recv()).await;
    match push_res {
        Ok(r) => match r {
            Ok(zmq_msg) => Ok(zmq_msg),
            Err(e) => Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                e.to_string(),
            ))),
        },
        Err(_e) => Err(GenericError::ZMQTimeoutError),
    }
}

pub fn format_zmq_msg_str(args: Vec<&str>) -> String {
    let mut zmq_str = String::new();
    match args.len() {
        0 => zmq_str,
        1 => {
            zmq_str.push_str(args[0]);
            zmq_str
        }
        _ => {
            zmq_str.push_str(args[0]);
            for arg in &args[1..] {
                zmq_str.push(ZMQ_MESSAGE_DELIMITER as char);
                zmq_str.push_str(arg);
            }
            zmq_str
        }
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
            Err(_e) => Err(GenericError::ZMQTimeoutError),
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
        assert!(
            send_recv_with_timeout(
                endpoint,
                ZmqMessage::from("hello"),
                Duration::from_millis(1)
            )
            .await
            .is_err()
        )
    }

    #[test]
    fn test_get_agent_tcp_uri() {
        let host = "localhost";
        let port = 1234 as usize;
        assert_eq!(get_server_tcp_uri(host, port), "tcp://localhost:1234")
    }

    #[tokio::test]
    async fn test_push_with_timeout_good() {
        let host = String::from("0.0.0.0");
        let port = 9995;
        let endpoint = get_server_tcp_uri(&host, port);
        let mut pull = get_zmq_pull(&endpoint).await.unwrap();
        let mut push = get_zmq_push(&endpoint).await.unwrap();
        tokio::spawn(async move {
            let msg = pull.recv().await.unwrap();
            assert_eq!(String::try_from(msg).unwrap(), "OK")
        });
        assert!(
            push_with_timeout(&mut push, Duration::from_secs(1), "OK".into())
                .await
                .is_ok()
        )
    }

    #[tokio::test]
    async fn test_create_push_with_timeout_bad() {
        let host = String::from("0.0.0.0");
        let port = 9995;
        let endpoint = get_server_tcp_uri(&host, port);
        // push created before pull so won't connect properly to pull-bound port
        assert!(get_zmq_push(&endpoint).await.is_err())
    }

    #[test]
    fn test_format_zmq_msg() {
        assert_eq!(
            format_zmq_msg_str(vec!["abc1", "de1f"]),
            String::from_utf8(b"abc1\x01de1f".to_vec()).unwrap()
        )
    }
}
