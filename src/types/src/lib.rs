use async_trait::async_trait;
use std::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum Exchange {
    LBank,
    Bitrue,
}

#[async_trait]
pub trait ExchangeSubscriber {
    async fn run_async(
        self,
        sender: Sender<OrderbookUpdate>,
        cancellation_token: CancellationToken,
    );
}

#[derive(Debug)]
pub struct OrderbookUpdate {
    pub timestamp_ms: u64,
    pub best_ask: Order,
    pub best_bid: Order,
}

#[derive(Debug)]
pub struct Order {
    pub price: f64,
    pub amount: u128,
}
