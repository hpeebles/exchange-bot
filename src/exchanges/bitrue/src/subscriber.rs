use crate::serialize_to_json;
use async_trait::async_trait;
use ezsockets::client::ClientCloseMode;
use ezsockets::{ClientConfig, ClientExt, Error, MessageSignal, WSError};
use flate2::bufread::GzDecoder;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::str::FromStr;
use std::sync::mpsc::Sender;
use tokio::select;
use tokio_util::sync::CancellationToken;
use xb_types::{ExchangeSubscriber, Order, OrderbookUpdate};

const URL: &str = "wss://ws.bitrue.com/market/ws";

#[derive(Default)]
pub struct BitrueSubscriber {}

struct WebSocketClient {
    handle: ezsockets::Client<Self>,
    sender: Sender<OrderbookUpdate>,
}

impl WebSocketClient {
    fn send<S: Serialize>(&mut self, value: &S) -> Result<MessageSignal, Error> {
        let json = serialize_to_json(&value);
        println!("Bitrue: Sending message: {json}");
        self.handle.text(json).map_err(|e| e.into())
    }

    fn subscribe(&mut self) -> Result<MessageSignal, Error> {
        self.send(&Subscribe {
            event: "sub".to_string(),
            params: SubscribeParams {
                cb_id: "chatusdt".to_string(),
                channel: "market_chatusdt_simple_depth_step0".to_string(),
            },
        })
    }
}

#[async_trait]
impl ClientExt for WebSocketClient {
    type Call = ();

    async fn on_text(&mut self, _: String) -> Result<(), Error> {
        unreachable!()
    }

    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        let mut d = GzDecoder::new(bytes.as_slice());
        let mut s = String::new();
        d.read_to_string(&mut s).unwrap();
        println!("Bitrue: Received text: {s}");

        if let Ok(m) = serde_json::from_str::<MarketDepth>(&s) {
            let update = OrderbookUpdate {
                timestamp_ms: m.timestamp,
                best_bid: Order {
                    price: f64::from_str(&m.tick.buys[0][0]).unwrap(),
                    amount: (f64::from_str(&m.tick.buys[0][1]).unwrap() * 1_0000_0000f64) as u128,
                },
                best_ask: Order {
                    price: f64::from_str(&m.tick.asks[0][0]).unwrap(),
                    amount: (f64::from_str(&m.tick.asks[0][1]).unwrap() * 1_0000_0000f64) as u128,
                },
            };
            self.sender.send(update).unwrap();
        } else if let Ok(Ping { ping }) = serde_json::from_str(&s) {
            self.send(&Pong { pong: ping }).unwrap();
        }
        Ok(())
    }

    async fn on_call(&mut self, _: Self::Call) -> Result<(), Error> {
        unreachable!()
    }

    async fn on_connect(&mut self) -> Result<(), Error> {
        println!("Bitrue: Connected");
        self.subscribe().unwrap();
        Ok(())
    }

    async fn on_disconnect(&mut self) -> Result<ClientCloseMode, Error> {
        println!("Bitrue: Disconnected");
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_connect_fail(&mut self, error: WSError) -> Result<ClientCloseMode, Error> {
        println!("Bitrue: Failed to connect: {error:?}");
        Err(error.into())
    }
}

#[async_trait]
impl ExchangeSubscriber for BitrueSubscriber {
    async fn run_async(
        self,
        sender: Sender<OrderbookUpdate>,
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

        println!("Bitrue disconnected");
    }
}

#[derive(Serialize, Deserialize)]
struct Ping {
    ping: u64,
}

#[derive(Serialize, Deserialize)]
struct Pong {
    pong: u64,
}

#[derive(Serialize, Deserialize)]
struct Subscribe {
    event: String,
    params: SubscribeParams,
}

#[derive(Serialize, Deserialize)]
struct SubscribeParams {
    cb_id: String,
    channel: String,
}

#[derive(Serialize, Deserialize)]
struct MarketDepth {
    channel: String,
    tick: MarketDepthInner,
    #[serde(rename = "ts")]
    timestamp: u64,
}

#[derive(Serialize, Deserialize)]
struct MarketDepthInner {
    buys: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}
