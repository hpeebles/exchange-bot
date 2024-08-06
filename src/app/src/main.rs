use std::io;
use tokio_util::sync::CancellationToken;
use tracing::info;
use xb_subscriber::Subscriber;
use xb_types::Exchange;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_writer(io::stdout).init();

    info!("Service started");

    abort_on_panic();

    let shutdown = CancellationToken::new();
    let subscriber = Subscriber::new(vec![Exchange::Bitrue, Exchange::LBank]);

    let _rx = subscriber.run(shutdown.clone());

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
