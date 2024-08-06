use std::cmp::min;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::info;
use xb_types::{
    Amount8Decimals, Exchange, OrderbookState, OrderbookStateProcessor, PendingMarketOrder,
    Price4Decimals,
};

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

pub struct Cashout {
    average_interval: Duration,
    amount_to_sell_per_iteration: Amount8Decimals,
    order_sender: Sender<Arc<PendingMarketOrder>>,
    asks_per_exchange: HashMap<Exchange, BTreeMap<Price4Decimals, Amount8Decimals>>,
}

impl Cashout {
    pub fn new(
        average_interval: Duration,
        amount_per_day: Amount8Decimals,
        order_sender: Sender<Arc<PendingMarketOrder>>,
    ) -> Cashout {
        let amount_to_sell_per_iteration = Amount8Decimals::from_units(
            amount_per_day.units() * average_interval.as_millis() / ONE_DAY.as_millis(),
        );

        Cashout {
            average_interval,
            amount_to_sell_per_iteration,
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
                        .map(|(e, a)| (*e, self.calculate_return(a)))
                        .max_by_key(|(_, r)| *r)
                    {
                        let order = PendingMarketOrder {
                            exchange,
                            amount: self.amount_to_sell_per_iteration,
                            expected_return,
                        };
                        info!("Cashout: {order:?}");
                        self.order_sender.send(Arc::new(order)).unwrap();
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

    fn calculate_return(
        &self,
        asks: &BTreeMap<Price4Decimals, Amount8Decimals>,
    ) -> Amount8Decimals {
        let mut total_return = 0;
        let mut total_remaining = self.amount_to_sell_per_iteration.units();

        for (price, amount) in asks {
            let amount_units = min(amount.units(), total_remaining);
            total_return += amount_units * price.units() / 1_0000;
            total_remaining -= amount_units;

            if total_remaining == 0 {
                break;
            }
        }

        Amount8Decimals::from_units(total_return)
    }
}

impl OrderbookStateProcessor for Cashout {
    fn run(self, updates: Receiver<Arc<OrderbookState>>, cancellation_token: CancellationToken) {
        tokio::spawn(self.run_async(updates, cancellation_token));
    }
}
