use std::sync::Arc;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio_util::sync::CancellationToken;
use xb_exchanges_bitrue::BitrueSubscriber;
use xb_exchanges_lbank::LBankSubscriber;
use xb_types::{Exchange, ExchangeSubscriber, OrderbookState};

pub struct Subscriber {
    exchanges: Vec<Exchange>,
}

pub struct SubscriptionManager {
    orderbook_state: Receiver<Arc<OrderbookState>>,
}

impl Subscriber {
    pub fn new(exchanges: Vec<Exchange>) -> Subscriber {
        Subscriber { exchanges }
    }

    pub fn run(self, cancellation_token: CancellationToken) -> SubscriptionManager {
        let (sender, receiver) = channel(1024);

        tokio::spawn(self.run_async(sender, cancellation_token));

        SubscriptionManager {
            orderbook_state: receiver,
        }
    }

    async fn run_async(
        self,
        sender: Sender<Arc<OrderbookState>>,
        cancellation_token: CancellationToken,
    ) {
        let mut futures = Vec::new();
        for exchange in self.exchanges {
            match exchange {
                Exchange::Bitrue => {
                    let bitrue_service = BitrueSubscriber::default();
                    futures
                        .push(bitrue_service.run_async(sender.clone(), cancellation_token.clone()));
                }
                Exchange::LBank => {
                    let lbank_service = LBankSubscriber::default();
                    futures
                        .push(lbank_service.run_async(sender.clone(), cancellation_token.clone()));
                }
            }
        }

        futures::future::select_all(futures).await;
    }
}

impl SubscriptionManager {
    pub fn subscribe_orderbook_state(&self) -> Receiver<Arc<OrderbookState>> {
        self.orderbook_state.resubscribe()
    }
}
