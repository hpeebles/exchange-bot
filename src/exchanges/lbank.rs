#![allow(dead_code)]
use crate::{ExchangeSubscriber, OrderbookUpdate};
use async_trait::async_trait;
use ezsockets::{ClientExt, Error};
use std::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

const URL: &str = "wss://www.lbkex.net/ws/V2/";

pub struct LBankSubscriber {
    config: LBankConfig,
    sender: Sender<OrderbookUpdate>,
}

pub struct LBankConfig {
    pub api_key: String,
    pub secret_key: String,
}

struct WebSocketClient {
    handle: ezsockets::Client<Self>,
}

#[async_trait]
impl ClientExt for WebSocketClient {
    type Call = ();

    async fn on_text(&mut self, _text: String) -> Result<(), Error> {
        todo!()
    }

    async fn on_binary(&mut self, _bytes: Vec<u8>) -> Result<(), Error> {
        todo!()
    }

    async fn on_call(&mut self, _call: Self::Call) -> Result<(), Error> {
        todo!()
    }
}

#[async_trait]
impl ExchangeSubscriber<LBankConfig> for LBankSubscriber {
    fn new(config: LBankConfig, sender: Sender<OrderbookUpdate>) -> Self {
        LBankSubscriber { config, sender }
    }

    async fn run_async(self, _cancellation_token: CancellationToken) {}
}
