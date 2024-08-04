#![allow(dead_code)]
pub mod exchanges;

use async_trait::async_trait;
use std::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait ExchangeSubscriber<C> {
    fn new(config: C, sender: Sender<OrderbookUpdate>) -> Self;
    async fn run_async(self, cancellation_token: CancellationToken);
}

pub struct OrderbookUpdate {
    timestamp_ms: u64,
    best_ask: Order,
    best_bid: Order,
}

struct Order {
    price: u128,
    amount: u128,
}

struct OrderProcessor {
    receiver: Receiver<OrderbookUpdate>,
}
