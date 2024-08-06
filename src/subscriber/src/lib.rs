use std::sync::Arc;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio_util::sync::CancellationToken;
use xb_exchanges_bitrue::BitrueSubscriber;
use xb_exchanges_lbank::LBankSubscriber;
use xb_types::{Exchange, ExchangeSubscriber, OrderbookState};

pub struct Subscriber {
    exchanges: Vec<Exchange>,
}

impl Subscriber {
    pub fn new(exchanges: Vec<Exchange>) -> Subscriber {
        Subscriber { exchanges }
    }

    pub fn run(self, cancellation_token: CancellationToken) -> Receiver<Arc<OrderbookState>> {
        let (sender, receiver) = channel(1024);

        tokio::spawn(self.run_async(sender, cancellation_token));

        receiver
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
