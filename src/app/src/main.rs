use tokio_util::sync::CancellationToken;
use xb_subscriber::Subscriber;
use xb_types::Exchange;

#[tokio::main]
async fn main() {
    println!("Service started");

    abort_on_panic();

    let shutdown = CancellationToken::new();
    let subscriber = Subscriber::new(vec![Exchange::Bitrue, Exchange::LBank]);

    let _rx = subscriber.run(shutdown.clone());

    tokio::signal::ctrl_c().await.unwrap();

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
