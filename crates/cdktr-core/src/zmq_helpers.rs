use std::time::Duration;

use crate::{
    exceptions::{GenericError, ZMQParseError},
    macros,
    utils::get_default_zmq_timeout,
};
use log::{info, error, warn};
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};
use zeromq::{
    DealerSocket, PubSocket, PullSocket, PushSocket, RepSocket, ReqSocket, RouterSocket, Socket,
    SocketRecv, SocketSend, SubSocket, ZmqMessage,
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

/// Creates a DEALER socket and connects to the given URI.
/// For use as the client side of a long-lived DEALER-ROUTER connection.
pub async fn get_zmq_dealer(endpoint_uri: &str) -> Result<DealerSocket, GenericError> {
    let mut dealer = DealerSocket::new();
    dealer
        .connect(endpoint_uri)
        .await
        .map_err(|e| GenericError::ZMQError(e.to_string()))?;
    Ok(dealer)
}

/// Creates a ROUTER socket and binds to the given URI.
/// For use as the server side of a DEALER-ROUTER fan-in.
pub async fn get_zmq_router(endpoint_uri: &str) -> Result<RouterSocket, GenericError> {
    let mut router = RouterSocket::new();
    router
        .bind(endpoint_uri)
        .await
        .map_err(|e| GenericError::ZMQParseError(ZMQParseError::ParseError(e.to_string())))?;
    Ok(router)
}

/// Send a single ZMQ message and receive a reply using an ephemeral DEALER socket.
/// Functionally equivalent to send_recv_with_timeout but uses DEALER instead of REQ,
/// which means the connection is no longer tracked by the server after the response,
/// avoiding file-descriptor accumulation on the server side.
pub async fn send_recv_dealer_with_timeout(
    tcp_uri: String,
    zmq_msg: ZmqMessage,
    duration: Duration,
) -> Result<ZmqMessage, GenericError> {
    let join_res = tokio::spawn(timeout(duration, async move {
        let mut dealer = get_zmq_dealer(&tcp_uri).await?;
        dealer
            .send(zmq_msg)
            .await
            .map_err(|e| GenericError::ZMQError(e.to_string()))?;
        dealer
            .recv()
            .await
            .map_err(|e| GenericError::ZMQError(e.to_string()))
    }))
    .await;

    match join_res {
        Ok(time_r) => match time_r {
            Ok(zmq_r) => zmq_r,
            Err(_e) => Err(GenericError::ZMQTimeoutError),
        },
        Err(e) => Err(GenericError::RuntimeError(e.to_string())),
    }
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

type ConnectionRequest = (ZmqMessage, oneshot::Sender<Result<ZmqMessage, GenericError>>);

/// A cheaply-cloneable handle to a persistent DEALER socket actor task.
///
/// Each `PrincipalConnection` clones the same `mpsc::Sender`, meaning all
/// callers share a single long-lived underlying `DealerSocket`. Requests are
/// serialised through the channel so the socket never has concurrent in-flight
/// messages.
#[derive(Clone)]
pub struct PrincipalConnection {
    tx: mpsc::Sender<ConnectionRequest>,
}

impl PrincipalConnection {
    pub fn new(host: &str, port: usize) -> Self {
        Self::new_from_uri(get_server_tcp_uri(host, port))
    }

    /// Create a connection from a full TCP URI string (e.g. "tcp://127.0.0.1:5555").
    pub fn new_from_uri(uri: String) -> Self {
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(connection_actor(uri, rx));
        Self { tx }
    }

    /// Send a request and await the reply using the persistent DEALER socket.
    pub async fn request(&self, msg: ZmqMessage) -> Result<ZmqMessage, GenericError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send((msg, reply_tx))
            .await
            .map_err(|_| GenericError::ZMQError("Connection actor has stopped".to_string()))?;
        reply_rx
            .await
            .map_err(|_| GenericError::ZMQError("Connection actor reply channel closed".to_string()))?
    }
}

async fn connection_actor(uri: String, mut rx: mpsc::Receiver<ConnectionRequest>) {
    let mut dealer: Option<DealerSocket> = None;

    while let Some((msg, reply_tx)) = rx.recv().await {
        if dealer.is_none() {
            info!("(Re)connecting DEALER to {}", uri);
            match get_zmq_dealer(&uri).await {
                Ok(d) => dealer = Some(d),
                Err(e) => {
                    error!("Failed to connect DEALER to {}: {}", uri, e);
                    let _ = reply_tx.send(Err(e));
                    continue;
                }
            }
        }

        let result = send_recv_on_dealer(dealer.as_mut().unwrap(), msg).await;
        if result.is_err() {
            warn!("DEALER error — invalidating socket for clean retry");
            dealer = None;
        }
        let _ = reply_tx.send(result);
    }
}

async fn send_recv_on_dealer(
    dealer: &mut DealerSocket,
    msg: ZmqMessage,
) -> Result<ZmqMessage, GenericError> {
    let duration = get_default_zmq_timeout();
    match timeout(duration, dealer.send(msg)).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return Err(GenericError::ZMQError(e.to_string())),
        Err(_) => return Err(GenericError::ZMQTimeoutError),
    }
    match timeout(duration, dealer.recv()).await {
        Ok(Ok(reply)) => Ok(reply),
        Ok(Err(e)) => Err(GenericError::ZMQError(e.to_string())),
        Err(_) => Err(GenericError::ZMQTimeoutError),
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
