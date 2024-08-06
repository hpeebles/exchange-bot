use std::io;
use tokio::sync::broadcast::channel;
use tokio_util::sync::CancellationToken;
use tracing::info;
use xb_arb_finder::ArbFinder;
use xb_subscriber::Subscriber;
use xb_types::{Exchange, OrderbookStateProcessor};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_writer(io::stdout).init();

    info!("Service started");

    abort_on_panic();

    let shutdown = CancellationToken::new();
    let subscriber = Subscriber::new(vec![Exchange::Bitrue, Exchange::LBank]);

    let (arb_tx, _arb_rx) = channel(1024);

    let arb_finder = ArbFinder::new(arb_tx);

    let rx = subscriber.run(shutdown.clone());

    arb_finder.run(rx, shutdown.clone());

    tokio::signal::ctrl_c().await.unwrap();

    info!("Service stopping");
    shutdown.cancel();
    info!("Service stopped");
}

pub fn abort_on_panic() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        default_hook(panic_info);
        std::process::abort();
    }));
}
