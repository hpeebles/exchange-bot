#![allow(dead_code)]
use crate::{ExchangeSubscriber, OrderbookUpdate};
use std::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

pub struct LBankSubscriber {
    config: LBankConfig,
    sender: Sender<OrderbookUpdate>,
    cancellation_token: CancellationToken,
}

pub struct LBankConfig {
    pub api_key: String,
    pub secret_key: String,
}

impl ExchangeSubscriber<LBankConfig> for LBankSubscriber {
    fn new(config: LBankConfig, sender: Sender<OrderbookUpdate>, cancellation_token: CancellationToken) -> Self {
        LBankSubscriber {
            config,
            sender,
            cancellation_token,
        }
    }

    fn start(self) {}
}
