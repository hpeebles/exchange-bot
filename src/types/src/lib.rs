use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Exchange {
    LBank,
    Bitrue,
}

#[async_trait]
pub trait ExchangeSubscriber {
    async fn run_async(
        self,
        sender: Sender<Arc<OrderbookState>>,
        cancellation_token: CancellationToken,
    );
}

#[async_trait]
pub trait ExchangeOrderExecutor: Send {
    async fn submit_order(&self, order: PendingOrder) -> Result<String, String>;
}

pub trait OrderbookStateProcessor {
    fn run(self, updates: Receiver<Arc<OrderbookState>>, cancellation_token: CancellationToken);
}

#[derive(Clone, Debug)]
pub struct OrderbookState {
    pub exchange: Exchange,
    pub timestamp_ms: u64,
    pub asks: BTreeMap<Decimal, Decimal>,
    pub bids: BTreeMap<Decimal, Decimal>,
}

impl OrderbookState {
    pub fn best_bid(&self) -> Option<Order> {
        self.bids.iter().next_back().map(|(p, a)| Order {
            exchange: self.exchange,
            price: *p,
            amount: *a,
        })
    }

    pub fn best_ask(&self) -> Option<Order> {
        self.asks.iter().next().map(|(p, a)| Order {
            exchange: self.exchange,
            price: *p,
            amount: *a,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ArbOpportunity {
    pub buy: Order,
    pub sell: Order,
}

#[derive(Clone, Debug)]
pub struct Order {
    pub exchange: Exchange,
    pub price: Decimal,
    pub amount: Decimal,
}

#[derive(Clone, Debug)]
pub enum PendingOrder {
    Limit(PendingLimitOrder),
    Market(PendingMarketOrder),
}

impl PendingOrder {
    pub fn exchange(&self) -> Exchange {
        match self {
            PendingOrder::Limit(o) => o.exchange,
            PendingOrder::Market(o) => o.exchange,
        }
    }

    pub fn direction(&self) -> Direction {
        match self {
            PendingOrder::Limit(o) => o.direction,
            PendingOrder::Market(o) => o.direction,
        }
    }

    pub fn amount(&self) -> Decimal {
        match self {
            PendingOrder::Limit(o) => o.amount,
            PendingOrder::Market(o) => o.amount,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PendingLimitOrder {
    pub exchange: Exchange,
    pub direction: Direction,
    pub amount: Decimal,
    pub price: Decimal,
}

#[derive(Clone, Debug)]
pub struct PendingMarketOrder {
    pub exchange: Exchange,
    pub direction: Direction,
    pub amount: Decimal,
    pub expected_return: Decimal,
}

#[derive(Copy, Clone, Debug)]
pub enum Direction {
    Buy,
    Sell,
}

impl Direction {
    pub fn is_buy(&self) -> bool {
        matches!(self, Direction::Buy)
    }
}
