use arby::exchanges::lbank::{LBankConfig, LBankSubscriber};
use arby::ExchangeSubscriber;
use std::sync::mpsc::channel;
use tokio::select;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    println!("Service started");

    abort_on_panic();

    let (tx, _rx) = channel();
    let shutdown = CancellationToken::new();

    select! {
        _ = {
            let config = LBankConfig {
                api_key: "123".to_string(),
                secret_key: "xyz".to_string(),
            };

            let lbank_service = LBankSubscriber::new(config, tx.clone());
            lbank_service.run_async(shutdown.clone())
        } => (),
        _ = tokio::signal::ctrl_c() => {
            println!("Ctrl-c received");
        },
    }

    println!("Service stopping");
    shutdown.cancel();
    println!("Service stopped");
}

pub fn abort_on_panic() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        default_hook(panic_info);
        std::process::abort();
    }));
}
