use exchange_bot::exchanges::bitrue::BitrueSubscriber;
use exchange_bot::exchanges::lbank::LBankSubscriber;
use exchange_bot::ExchangeSubscriber;
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
            let lbank_service = LBankSubscriber::new(tx.clone());
            lbank_service.run_async(shutdown.clone())
        } => (),
        _ = {
            let bitrue_service = BitrueSubscriber::new(tx.clone());
            bitrue_service.run_async(shutdown.clone())
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
