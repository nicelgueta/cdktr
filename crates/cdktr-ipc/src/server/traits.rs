use async_trait::async_trait;
use cdktr_api::models::ClientResponseMessage;
use cdktr_core::exceptions::GenericError;
use cdktr_core::zmq_helpers::{get_server_tcp_uri, get_zmq_router};
use log::{error, info, warn};

use zeromq::{ZmqMessage};
use zeromq::{SocketRecv, SocketSend};

/// Server trait implemented by the principal to handle incoming requests.
///
/// The underlying transport uses a **ROUTER socket** (replaces the previous REP socket),
/// which allows multiple persistent DEALER clients to multiplex requests over long-lived
/// connections. This eliminates the file-descriptor leak caused by the old pattern of
/// creating a new REQ socket per request.
///
/// Frame protocol (DEALER → ROUTER, no empty delimiter):
/// - Incoming: `[identity_frame][data_frame]`
/// - Outgoing: `[identity_frame][reply_frame]`
///
/// The identity stripping/prepending is handled internally in `start()`. Implementors
/// only see the data frame via `handle_client_message`.
#[async_trait]
pub trait Server<RT>
where
    RT: TryFrom<ZmqMessage, Error = GenericError> + Send,
{
    /// Handle a decoded client request and return a response plus an exit code.
    /// A non-zero exit code causes the server loop to stop and return that code.
    async fn handle_client_message(&mut self, cli_msg: RT) -> (ClientResponseMessage, usize);

    /// Bind a ROUTER socket and run the request-handling loop until a non-zero
    /// exit code is returned by `handle_client_message`.
    async fn start(&mut self, current_host: &str, port: usize) -> Result<usize, GenericError> {
        info!(
            "SERVER: Starting ROUTER server on tcp://{}:{}",
            current_host, port
        );
        let mut router = get_zmq_router(&get_server_tcp_uri(current_host, port)).await?;
        info!("SERVER: ROUTER socket bound successfully");

        let exit_code = loop {
            let mut zmq_recv = match router.recv().await {
                Ok(msg) => msg,
                Err(e) => {
                    error!("ROUTER recv error: {}", e);
                    continue;
                }
            };

            // ROUTER prepends the peer identity as the first frame.
            // split_off(1) leaves only [identity] in zmq_recv and returns [data...] as data_msg.
            let data_msg = zmq_recv.split_off(1);
            let identity = zmq_recv; // now contains exactly [identity_frame]

            let msg_res: Result<RT, GenericError> = RT::try_from(data_msg);
            let (response, exit_code) = match msg_res {
                Ok(cli_msg) => self.handle_client_message(cli_msg).await,
                Err(e) => {
                    warn!("Failed to parse client message: {}", e);
                    (ClientResponseMessage::ClientError(e.to_string()), 0)
                }
            };

            // Prepend the identity so ROUTER can route the reply back to the originating DEALER.
            let mut reply_msg: ZmqMessage = response.into();
            reply_msg.prepend(&identity);
            if let Err(e) = router.send(reply_msg).await {
                error!("ROUTER send error: {}", e);
            }

            if exit_code > 0 {
                break exit_code;
            }
        };
        Ok(exit_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::{DealerSocket, Socket, SocketRecv, SocketSend};

    /// Newtype so we can implement TryFrom<ZmqMessage> without conflicting with the
    /// blanket impl that already exists in the zeromq crate for String.
    struct TestCmd(String);

    impl TryFrom<ZmqMessage> for TestCmd {
        type Error = GenericError;
        fn try_from(msg: ZmqMessage) -> Result<Self, Self::Error> {
            String::try_from(msg)
                .map(TestCmd)
                .map_err(|e| GenericError::ZMQError(e.to_string()))
        }
    }

    struct EchoServer;

    #[async_trait::async_trait]
    impl Server<TestCmd> for EchoServer {
        async fn handle_client_message(
            &mut self,
            cli_msg: TestCmd,
        ) -> (ClientResponseMessage, usize) {
            // Echo the payload back; exit on "QUIT"
            let exit = if cli_msg.0 == "QUIT" { 1 } else { 0 };
            (
                ClientResponseMessage::SuccessWithPayload(cli_msg.0),
                exit,
            )
        }
    }

    #[tokio::test]
    async fn test_router_server_request_reply() {
        let port = 15_123usize;

        tokio::spawn(async move {
            let mut server = EchoServer;
            server.start("127.0.0.1", port).await.unwrap();
        });

        // Give the server time to bind
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut dealer = DealerSocket::new();
        dealer
            .connect(&format!("tcp://127.0.0.1:{}", port))
            .await
            .unwrap();

        // PING → expect SuccessWithPayload("PING") on the wire ("SUCCESS\x01PING")
        dealer.send(ZmqMessage::from("PING")).await.unwrap();
        let reply = dealer.recv().await.unwrap();
        let reply_str = String::try_from(reply).unwrap();
        assert!(
            reply_str.contains("PING"),
            "Expected reply to contain PING, got: {}",
            reply_str
        );

        // Send QUIT to trigger server exit
        dealer.send(ZmqMessage::from("QUIT")).await.unwrap();
        let _ = dealer.recv().await;
    }
}

