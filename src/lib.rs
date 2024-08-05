pub mod exchanges;

use async_trait::async_trait;
use std::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait ExchangeSubscriber<C> {
    fn new(config: C, sender: Sender<OrderbookUpdate>) -> Self;
    async fn run_async(self, cancellation_token: CancellationToken);
}

#[derive(Debug)]
pub struct OrderbookUpdate {
    pub timestamp_ms: u64,
    pub best_ask: Order,
    pub best_bid: Order,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Order {
    price: f64,
    amount: u128,
}

#[allow(dead_code)]
struct OrderProcessor {
    receiver: Receiver<OrderbookUpdate>,
}
