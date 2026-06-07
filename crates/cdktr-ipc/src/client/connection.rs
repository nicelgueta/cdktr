// PrincipalConnection lives in cdktr-core so that cdktr-events can also use it
// (cdktr-events cannot depend on cdktr-ipc due to the dependency chain).
pub use cdktr_core::zmq_helpers::PrincipalConnection;

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::{Endpoint, RouterSocket, Socket, SocketRecv, SocketSend, ZmqMessage};

    async fn create_echo_router() -> (RouterSocket, usize) {
        let mut router = RouterSocket::new();
        let ep = router.bind("tcp://127.0.0.1:0").await.unwrap();
        let port = if let Endpoint::Tcp(_, p) = ep {
            p as usize
        } else {
            panic!("unexpected endpoint type")
        };
        (router, port)
    }

    fn run_echo_router(mut router: RouterSocket, count: usize) {
        tokio::spawn(async move {
            for _ in 0..count {
                let mut recv = router.recv().await.unwrap();
                // recv = [identity_frame, data_frame]; split_off(1) separates them
                let data = recv.split_off(1); // recv = [identity], data = [payload]
                let mut reply = data;
                reply.prepend(&recv); // reply = [identity, payload]
                router.send(reply).await.unwrap();
            }
        });
    }

    /// Spin up a ROUTER echo server and verify a single request round-trips
    #[tokio::test]
    async fn test_connection_request_reply() {
        let (router, port) = create_echo_router().await;
        run_echo_router(router, 1);

        let conn = PrincipalConnection::new("127.0.0.1", port);
        let response = conn.request(ZmqMessage::from("PING")).await.unwrap();
        assert_eq!(String::try_from(response).unwrap(), "PING");
    }

    /// Verify that a cloned PrincipalConnection shares the same actor (two handles, one socket)
    #[tokio::test]
    async fn test_connection_clone_shares_actor() {
        let (router, port) = create_echo_router().await;
        run_echo_router(router, 2);

        let conn1 = PrincipalConnection::new("127.0.0.1", port);
        let conn2 = conn1.clone();

        let r1 = conn1.request(ZmqMessage::from("MSG1")).await.unwrap();
        let r2 = conn2.request(ZmqMessage::from("MSG2")).await.unwrap();
        assert_eq!(String::try_from(r1).unwrap(), "MSG1");
        assert_eq!(String::try_from(r2).unwrap(), "MSG2");
    }
}
