use std::collections::HashMap;
use std::sync::Arc;
use tokio::select;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::info;
use xb_types::{
    ArbOpportunity, Direction, Exchange, OrderbookState, OrderbookStateProcessor,
    PendingMarketOrder, PendingOrder,
};

pub struct ArbFinder {
    order_sender: Sender<Arc<PendingOrder>>,
    state_per_exchange: HashMap<Exchange, OrderbookState>,
}

impl ArbFinder {
    pub fn new(order_sender: Sender<Arc<PendingOrder>>) -> ArbFinder {
        ArbFinder {
            order_sender,
            state_per_exchange: HashMap::new(),
        }
    }

    async fn run_async(
        mut self,
        mut updates: Receiver<Arc<OrderbookState>>,
        cancellation_token: CancellationToken,
    ) {
        info!("ArbFinder started");

        loop {
            select! {
                next = updates.recv() => {
                    if let Ok(state) = next {
                        let exchange = state.exchange;
                        self.state_per_exchange.insert(exchange, (*state).clone());
                        self.find_and_notify_arbs(exchange);
                    }
                }
                _ = cancellation_token.cancelled() => break,
            }
        }

        info!("ArbFinder stopped");
    }

    fn find_and_notify_arbs(&self, latest_update: Exchange) {
        if let Some(updated) = self.state_per_exchange.get(&latest_update) {
            if let (Some(updated_bid), Some(updated_ask)) = (updated.best_bid(), updated.best_ask())
            {
                for existing in self
                    .state_per_exchange
                    .values()
                    .filter(|v| v.exchange != latest_update)
                {
                    if let Some(bid) = existing.best_bid() {
                        if bid.price > updated_ask.price {
                            let arb = ArbOpportunity {
                                buy: updated_ask.clone(),
                                sell: bid.clone(),
                            };
                            self.notify_arb(arb);
                        }
                    }

                    if let Some(ask) = existing.best_ask() {
                        if ask.price < updated_bid.price {
                            let arb = ArbOpportunity {
                                buy: ask.clone(),
                                sell: updated_bid.clone(),
                            };
                            self.notify_arb(arb);
                        }
                    }
                }
            }
        }
    }

    fn notify_arb(&self, arb: ArbOpportunity) {
        info!("Found arb: {arb:?}");

        self.order_sender
            .send(Arc::new(PendingOrder::Market(PendingMarketOrder {
                exchange: arb.sell.exchange,
                direction: Direction::Sell,
                amount: arb.sell.amount,
                expected_return: arb.sell.amount * arb.sell.price,
            })))
            .unwrap();

        self.order_sender
            .send(Arc::new(PendingOrder::Market(PendingMarketOrder {
                exchange: arb.buy.exchange,
                direction: Direction::Buy,
                amount: arb.buy.amount,
                expected_return: arb.buy.amount * arb.buy.price,
            })))
            .unwrap();
    }
}

impl OrderbookStateProcessor for ArbFinder {
    fn run(
        self,
        updates: Receiver<Arc<OrderbookState>>,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(self.run_async(updates, cancellation_token))
    }
}
