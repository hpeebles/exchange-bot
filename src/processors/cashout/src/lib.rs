use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::info;
use xb_types::{
    Direction, Exchange, OrderbookState, OrderbookStateProcessor, PendingMarketOrder, PendingOrder,
};

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

pub struct Cashout {
    average_interval: Duration,
    amount_to_sell_per_iteration: Decimal,
    min_price: Option<Decimal>,
    order_sender: Sender<Arc<PendingOrder>>,
    asks_per_exchange: HashMap<Exchange, BTreeMap<Decimal, Decimal>>,
}

impl Cashout {
    pub fn new(
        average_interval: Duration,
        amount_per_day: Decimal,
        min_price: Option<Decimal>,
        order_sender: Sender<Arc<PendingOrder>>,
    ) -> Cashout {
        let amount_to_sell_per_iteration = amount_per_day
            * Decimal::from(average_interval.as_millis())
            / Decimal::from(ONE_DAY.as_millis());

        Cashout {
            average_interval,
            amount_to_sell_per_iteration,
            min_price,
            order_sender,
            asks_per_exchange: HashMap::new(),
        }
    }

    async fn run_async(
        mut self,
        mut updates: Receiver<Arc<OrderbookState>>,
        cancellation_token: CancellationToken,
    ) {
        let sleep = tokio::time::sleep(self.next_duration());
        tokio::pin!(sleep);

        loop {
            select! {
                next = updates.recv() => {
                    if let Ok(state) = next {
                        let exchange = state.exchange;
                        self.asks_per_exchange.insert(exchange, state.asks.clone());
                    }
                }
                _ = &mut sleep => {
                    if let Some((exchange, expected_return)) = self
                        .asks_per_exchange
                        .iter()
                        .filter_map(|(e, a)| self.calculate_return(a).map(|r| (*e, r)))
                        .max_by_key(|(_, r)| *r)
                    {
                        let order = PendingMarketOrder {
                            exchange,
                            direction: Direction::Sell,
                            amount: self.amount_to_sell_per_iteration,
                            expected_return,
                        };
                        info!("Cashout: {order:?}");
                        self.order_sender.send(Arc::new(PendingOrder::Market(order))).unwrap();
                    } else {
                        info!("Cashout: No cashout available");
                    }
                    sleep.as_mut().reset(Instant::now() + self.next_duration());
                }
                _ = cancellation_token.cancelled() => break,
            }
        }
    }

    fn next_duration(&self) -> Duration {
        self.average_interval
    }

    fn calculate_return(&self, asks: &BTreeMap<Decimal, Decimal>) -> Option<Decimal> {
        let mut total_return = Decimal::ZERO;
        let mut total_remaining = self.amount_to_sell_per_iteration;

        for (&price, &amount) in asks {
            if let Some(min_price) = self.min_price {
                if price < min_price {
                    break;
                }
            }

            let amount = amount.min(total_remaining);
            total_return += amount * price;
            total_remaining -= amount;

            if total_remaining == Decimal::ZERO {
                return Some(total_return);
            }
        }

        None
    }
}

impl OrderbookStateProcessor for Cashout {
    fn run(self, updates: Receiver<Arc<OrderbookState>>, cancellation_token: CancellationToken) {
        tokio::spawn(self.run_async(updates, cancellation_token));
    }
}
