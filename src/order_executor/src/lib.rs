use std::collections::HashMap;
use std::sync::Arc;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::error;
use xb_types::{Exchange, ExchangeOrderExecutor, PendingOrder};

pub struct OrderExecutor {
    exchanges: HashMap<Exchange, Box<dyn ExchangeOrderExecutor>>,
}

pub struct OrderExecutorBuilder {
    exchanges: HashMap<Exchange, Box<dyn ExchangeOrderExecutor>>,
}

impl OrderExecutor {
    pub fn run(self, receiver: Receiver<Arc<PendingOrder>>, cancellation_token: CancellationToken) {
        tokio::spawn(self.run_async(receiver, cancellation_token));
    }

    async fn run_async(
        self,
        mut receiver: Receiver<Arc<PendingOrder>>,
        cancellation_token: CancellationToken,
    ) {
        loop {
            select! {
                next = receiver.recv() => {
                    if let Ok(order) = next {
                        let exchange = order.exchange();
                        if let Some(order_executor) = self.exchanges.get(&exchange) {
                            let _ = order_executor.submit_order((*order).clone()).await;
                        } else {
                            error!("No order executor found for exchange: {exchange:?}")
                        }
                    }
                },
                _ = cancellation_token.cancelled() => {
                    break;
                }
            }
        }
    }
}

impl OrderExecutorBuilder {
    pub fn new() -> OrderExecutorBuilder {
        OrderExecutorBuilder {
            exchanges: HashMap::new(),
        }
    }

    pub fn with_exchange<E: ExchangeOrderExecutor + 'static>(
        mut self,
        exchange: Exchange,
        order_executor: E,
    ) -> Self {
        self.exchanges.insert(exchange, Box::new(order_executor));
        self
    }

    pub fn build(self) -> OrderExecutor {
        OrderExecutor {
            exchanges: self.exchanges,
        }
    }
}
