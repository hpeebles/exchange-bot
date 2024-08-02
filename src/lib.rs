#![allow(dead_code)]
pub mod exchanges;

use std::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

pub trait ExchangeSubscriber<C> {
    fn new(config: C, sender: Sender<OrderbookUpdate>, cancellation_token: CancellationToken) -> Self;
    fn start(self);
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
