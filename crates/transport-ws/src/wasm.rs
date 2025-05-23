use super::WsBackend;
use alloy_pubsub::PubSubConnect;
use alloy_transport::{utils::Spawnable, TransportErrorKind, TransportResult};
use futures::{
    sink::SinkExt,
    stream::{Fuse, StreamExt},
};
use serde_json::value::RawValue;
use std::time::Duration;
use ws_stream_wasm::{WsErr, WsMessage, WsMeta, WsStream};

/// Simple connection info for the websocket.
#[derive(Clone, Debug)]
pub struct WsConnect {
    /// The URL to connect to.
    url: String,
    /// Max number of retries before failing and exiting the connection.
    /// Default is 10.
    max_retries: u32,
    /// The interval between retries.
    /// Default is 3 seconds.
    retry_interval: Duration,
}

impl WsConnect {
    /// Creates a new websocket connection configuration.
    pub fn new<S: Into<String>>(url: S) -> Self {
        Self { url: url.into(), max_retries: 10, retry_interval: Duration::from_secs(3) }
    }

    /// Sets the max number of retries before failing and exiting the connection.
    /// Default is 10.
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the interval between retries.
    /// Default is 3 seconds.
    pub const fn with_retry_interval(mut self, retry_interval: Duration) -> Self {
        self.retry_interval = retry_interval;
        self
    }

    /// Get the URL string of the connection.
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl PubSubConnect for WsConnect {
    fn is_local(&self) -> bool {
        alloy_transport::utils::guess_local_url(&self.url)
    }

    async fn connect(&self) -> TransportResult<alloy_pubsub::ConnectionHandle> {
        let socket =
            WsMeta::connect(&self.url, None).await.map_err(TransportErrorKind::custom)?.1.fuse();

        let (handle, interface) = alloy_pubsub::ConnectionHandle::new();
        let backend = WsBackend { socket, interface };

        backend.spawn();

        Ok(handle.with_max_retries(self.max_retries).with_retry_interval(self.retry_interval))
    }
}

impl WsBackend<Fuse<WsStream>> {
    /// Handle a message from the websocket.
    #[expect(clippy::result_unit_err)]
    pub fn handle(&mut self, item: WsMessage) -> Result<(), ()> {
        match item {
            WsMessage::Text(text) => self.handle_text(&text),
            WsMessage::Binary(_) => {
                error!("Received binary message, expected text");
                Err(())
            }
        }
    }

    /// Send a message to the websocket.
    pub async fn send(&mut self, msg: Box<RawValue>) -> Result<(), WsErr> {
        self.socket.send(WsMessage::Text(msg.get().to_owned())).await
    }

    /// Spawn this backend on a loop.
    pub fn spawn(mut self) {
        let fut = async move {
            let mut errored = false;
            loop {
                // We bias the loop as follows
                // 1. New dispatch to server.
                // 2. Response or notification from server.
                // This ensures that keepalive is sent only if no other messages
                // have been sent in the last 10 seconds. And prioritizes new
                // dispatches over responses from the server. This will fail if
                // the client saturates the task with dispatches, but that's
                // probably not a big deal.
                tokio::select! {
                    biased;
                    // we've received a new dispatch, so we send it via
                    // websocket. We handle new work before processing any
                    // responses from the server.
                    inst = self.interface.recv_from_frontend() => {
                        match inst {
                            Some(msg) => {
                                if let Err(err) = self.send(msg).await {
                                    error!(%err, "WS connection error");
                                    errored = true;
                                    break
                                }
                            },
                            // dispatcher has gone away
                            None => {
                                break
                            },
                        }
                    },
                    resp = self.socket.next() => {
                        match resp {
                            Some(item) => {
                                errored = self.handle(item).is_err();
                                if errored { break }
                            },
                            None => {
                                error!("WS server has gone away");
                                errored = true;
                                break
                            },
                        }
                    }
                }
            }
            if errored {
                self.interface.close_with_error();
            }
        };
        fut.spawn_task();
    }
}
