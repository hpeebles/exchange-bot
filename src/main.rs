use std::sync::mpsc::channel;
use std::thread;
use tokio_util::sync::CancellationToken;
use arby::exchanges::lbank::{LBankConfig, LBankSubscriber};
use arby::ExchangeSubscriber;

#[tokio::main]
async fn main() {
    abort_on_panic();
    let (tx, _rx) = channel();
    let shutdown = CancellationToken::new();

    let mut join_handles = Vec::new();
    join_handles.push(thread::spawn(move || {
        let config = LBankConfig {
            api_key: "123".to_string(),
            secret_key: "xyz".to_string(),
        };

        let lbank_service = LBankSubscriber::new(config, tx.clone(), shutdown.clone());
        lbank_service.start()
    }));

    thread::park();
}

pub fn abort_on_panic() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        default_hook(panic_info);
        std::process::abort();
    }));
}
