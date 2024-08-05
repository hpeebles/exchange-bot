use crate::{ExchangeSubscriber, Order, OrderbookUpdate};
use async_trait::async_trait;
use ezsockets::client::ClientCloseMode;
use ezsockets::{ClientConfig, ClientExt, Error, MessageSignal, WSError};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::mpsc::Sender;
use tokio::select;
use tokio_util::sync::CancellationToken;

const URL: &str = "wss://www.lbkex.net/ws/V2/";

#[allow(dead_code)]
pub struct LBankSubscriber {
    config: LBankConfig,
    sender: Sender<OrderbookUpdate>,
}

#[allow(dead_code)]
pub struct LBankConfig {
    pub api_key: String,
    pub secret_key: String,
}

struct WebSocketClient {
    handle: ezsockets::Client<Self>,
    sender: Sender<OrderbookUpdate>,
}

impl WebSocketClient {
    fn send(&mut self, value: &Action) -> Result<MessageSignal, Error> {
        let json = serialize_to_json(&value);
        println!("LBank: Sending message: {json}");
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
        println!("LBank: Received text: {text}");
        if let Ok(message) = serde_json::from_str(&text) {
            match message {
                DataMessage::MarketDepth(d) => {
                    let update = OrderbookUpdate {
                        timestamp_ms: 0,
                        best_bid: Order {
                            price: f64::from_str(&d.depth.bids[0][0]).unwrap(),
                            amount: (f64::from_str(&d.depth.bids[0][1]).unwrap() * 1_0000_0000f64)
                                as u128,
                        },
                        best_ask: Order {
                            price: f64::from_str(&d.depth.asks[0][0]).unwrap(),
                            amount: (f64::from_str(&d.depth.asks[0][1]).unwrap() * 1_0000_0000f64)
                                as u128,
                        },
                    };
                    println!("LBank: Received update: {update:?}");
                    self.sender.send(update).unwrap();
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
        println!("LBank: Connected");
        self.subscribe().unwrap();
        Ok(())
    }

    async fn on_disconnect(&mut self) -> Result<ClientCloseMode, Error> {
        println!("LBank: Disconnected");
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_connect_fail(&mut self, error: WSError) -> Result<ClientCloseMode, Error> {
        println!("LBank: Failed to connect: {error:?}");
        Err(error.into())
    }
}

#[async_trait]
impl ExchangeSubscriber<LBankConfig> for LBankSubscriber {
    fn new(config: LBankConfig, sender: Sender<OrderbookUpdate>) -> Self {
        LBankSubscriber { config, sender }
    }

    async fn run_async(self, cancellation_token: CancellationToken) {
        let (handle, future) = ezsockets::connect(
            |handle| WebSocketClient {
                handle,
                sender: self.sender.clone(),
            },
            ClientConfig::new(URL),
        )
        .await;

        select! {
            _ = future => (),
            _ = cancellation_token.cancelled() => {
                handle.close(None).unwrap();
            }
        }

        println!("LBank disconnected");
    }
}

fn serialize_to_json<S: Serialize>(value: &S) -> String {
    serde_json::to_string(value).unwrap()
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
