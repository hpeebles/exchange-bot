use crate::serialize_to_json;
use async_trait::async_trait;
use ezsockets::client::ClientCloseMode;
use ezsockets::{ClientConfig, ClientExt, Error, MessageSignal, WSError};
use flate2::bufread::GzDecoder;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;
use tokio::select;
use tokio::sync::broadcast::Sender;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, trace};
use xb_types::{Exchange, ExchangeSubscriber, OrderbookState};

const URL: &str = "wss://ws.bitrue.com/market/ws";

#[derive(Default)]
pub struct BitrueSubscriber {}

struct WebSocketClient {
    handle: ezsockets::Client<Self>,
    sender: Sender<Arc<OrderbookState>>,
}

impl WebSocketClient {
    fn send<S: Serialize>(&mut self, value: &S) -> Result<MessageSignal, Error> {
        let json = serialize_to_json(&value);
        trace!("Bitrue: Sending message: {json}");
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
        trace!("Bitrue: Received text: {s}");

        if let Ok(m) = serde_json::from_str::<MarketDepth>(&s) {
            let update = OrderbookState {
                exchange: Exchange::Bitrue,
                timestamp_ms: m.timestamp,
                bids: m
                    .tick
                    .buys
                    .into_iter()
                    .map(|b| {
                        (
                            Decimal::from_str(&b[0]).unwrap(),
                            Decimal::from_str(&b[1]).unwrap(),
                        )
                    })
                    .collect(),
                asks: m
                    .tick
                    .asks
                    .into_iter()
                    .map(|a| {
                        (
                            Decimal::from_str(&a[0]).unwrap(),
                            Decimal::from_str(&a[1]).unwrap(),
                        )
                    })
                    .collect(),
            };
            self.sender.send(Arc::new(update)).unwrap();
        } else if let Ok(Ping { ping }) = serde_json::from_str(&s) {
            self.send(&Pong { pong: ping }).unwrap();
        }
        Ok(())
    }

    async fn on_call(&mut self, _: Self::Call) -> Result<(), Error> {
        unreachable!()
    }

    async fn on_connect(&mut self) -> Result<(), Error> {
        info!("Bitrue: Connected");
        self.subscribe().unwrap();
        Ok(())
    }

    async fn on_disconnect(&mut self) -> Result<ClientCloseMode, Error> {
        info!("Bitrue: Disconnected");
        Ok(ClientCloseMode::Reconnect)
    }

    async fn on_connect_fail(&mut self, error: WSError) -> Result<ClientCloseMode, Error> {
        error!("Bitrue: Failed to connect: {error:?}");
        Ok(ClientCloseMode::Reconnect)
    }
}

#[async_trait]
impl ExchangeSubscriber for BitrueSubscriber {
    async fn run_async(
        self,
        sender: Sender<Arc<OrderbookState>>,
        cancellation_token: CancellationToken,
    ) {
        info!("BitrueSubscriber started");

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

        info!("BitrueSubscriber stopped");
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
