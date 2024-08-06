use crate::serialize_to_json;
use async_trait::async_trait;
use ezsockets::client::ClientCloseMode;
use ezsockets::{ClientConfig, ClientExt, Error, MessageSignal, WSError};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::select;
use tokio::sync::broadcast::Sender;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, trace};
use xb_types::{Amount8Decimals, Exchange, ExchangeSubscriber, OrderbookState, Price4Decimals};

const URL: &str = "wss://www.lbkex.net/ws/V2/";

#[derive(Default)]
pub struct LBankSubscriber {}

struct WebSocketClient {
    handle: ezsockets::Client<Self>,
    sender: Sender<Arc<OrderbookState>>,
}

impl WebSocketClient {
    fn send(&mut self, value: &Action) -> Result<MessageSignal, Error> {
        let json = serialize_to_json(&value);
        trace!("LBank: Sending message: {json}");
        self.handle.text(json).map_err(|e| e.into())
    }

    fn subscribe(&mut self) -> Result<MessageSignal, Error> {
        self.send(&Action::Subscribe(Subscribe::MarketDepth(
            SubscribeMarketDepth {
                pair: "chat_usdt".to_string(),
                depth: "10".to_string(),
            },
        )))
    }
}

#[async_trait]
impl ClientExt for WebSocketClient {
    type Call = ();

    async fn on_text(&mut self, text: String) -> Result<(), Error> {
        trace!("LBank: Received text: {text}");

        if let Ok(message) = serde_json::from_str(&text) {
            match message {
                DataMessage::MarketDepth(d) => {
                    let update = OrderbookState {
                        exchange: Exchange::LBank,
                        timestamp_ms: 0,
                        bids: d
                            .depth
                            .bids
                            .into_iter()
                            .map(|b| {
                                (
                                    Price4Decimals::from_str(&b[0]).unwrap(),
                                    Amount8Decimals::from_str(&b[1]).unwrap(),
                                )
                            })
                            .collect(),
                        asks: d
                            .depth
                            .asks
                            .into_iter()
                            .map(|a| {
                                (
                                    Price4Decimals::from_str(&a[0]).unwrap(),
                                    Amount8Decimals::from_str(&a[1]).unwrap(),
                                )
                            })
                            .collect(),
                    };
                    trace!("LBank: Received update: {update:?}");
                    self.sender.send(Arc::new(update)).unwrap();
                }
            }
        } else if let Ok(Action::Ping(Ping { ping })) = serde_json::from_str(&text) {
            self.send(&Action::Pong(Pong { pong: ping })).unwrap();
        }
        Ok(())
    }

    async fn on_binary(&mut self, _: Vec<u8>) -> Result<(), Error> {
        unreachable!()
    }

    async fn on_call(&mut self, _: Self::Call) -> Result<(), Error> {
        unreachable!()
    }

    async fn on_connect(&mut self) -> Result<(), Error> {
        info!("LBank: Connected");
        self.subscribe().unwrap();
        Ok(())
    }

    async fn on_disconnect(&mut self) -> Result<ClientCloseMode, Error> {
        info!("LBank: Disconnected");
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_connect_fail(&mut self, error: WSError) -> Result<ClientCloseMode, Error> {
        error!("LBank: Failed to connect: {error:?}");
        Ok(ClientCloseMode::Reconnect)
    }
}

#[async_trait]
impl ExchangeSubscriber for LBankSubscriber {
    async fn run_async(
        self,
        sender: Sender<Arc<OrderbookState>>,
        cancellation_token: CancellationToken,
    ) {
        let (handle, future) = ezsockets::connect(
            |handle| WebSocketClient { handle, sender },
            ClientConfig::new(URL),
        )
        .await;

        select! {
            _ = future => (),
            _ = cancellation_token.cancelled() => {
                handle.close(None).unwrap();
            }
        }

        info!("LBank disconnected");
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DataMessage {
    #[serde(rename = "depth")]
    MarketDepth(MarketDepth),
}

#[derive(Serialize, Deserialize)]
struct MarketDepth {
    pair: String,
    depth: MarketDepthInner,
    #[serde(rename = "TS")]
    timestamp: String,
}

#[derive(Serialize, Deserialize)]
struct MarketDepthInner {
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Action {
    Ping(Ping),
    Pong(Pong),
    Subscribe(Subscribe),
}

#[derive(Serialize, Deserialize)]
struct Ping {
    ping: String,
}

#[derive(Serialize, Deserialize)]
struct Pong {
    pong: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "subscribe")]
enum Subscribe {
    #[serde(rename = "depth")]
    MarketDepth(SubscribeMarketDepth),
}

#[derive(Serialize, Deserialize)]
struct SubscribeMarketDepth {
    depth: String,
    pair: String,
}
