use std::io;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::broadcast::channel;
use tokio_util::sync::CancellationToken;
use tracing::info;
use xb_arb_finder::ArbFinder;
use xb_cashout::Cashout;
use xb_exchanges_bitrue::BitrueClient;
use xb_exchanges_lbank::LBankClient;
use xb_order_executor::OrderExecutorBuilder;
use xb_subscriber::Subscriber;
use xb_types::{Exchange, OrderbookStateProcessor};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    tracing_subscriber::fmt().with_writer(io::stdout).init();

    info!("Service started");

    abort_on_panic();

    let shutdown = CancellationToken::new();
    let mut exchanges = Vec::new();
    if is_enabled("BITRUE") {
        exchanges.push(Exchange::Bitrue);
    }
    if is_enabled("LBANK") {
        exchanges.push(Exchange::LBank);
    }

    let subscriber = Subscriber::new(exchanges);
    let subscription_manager = subscriber.run(shutdown.clone());

    let (order_tx, order_rx) = channel(1024);

    if is_enabled("ARB_FINDER") {
        let arb_finder = ArbFinder::new(order_tx.clone());
        arb_finder.run(
            subscription_manager.subscribe_orderbook_state(),
            shutdown.clone(),
        );
    }
    if is_enabled("CASHOUT") {
        if let Some(amount) = get_config("CASHOUT_AMOUNT_PER_DAY") {
            let cashout = Cashout::new(
                Duration::from_secs(get_config("CASHOUT_AVG_INTERVAL_SECS").unwrap_or(300)),
                amount,
                get_config("CASHOUT_MIN_PRICE"),
                order_tx.clone(),
            );
            cashout.run(
                subscription_manager.subscribe_orderbook_state(),
                shutdown.clone(),
            );
        }
    }

    if is_enabled("ORDER_EXECUTOR") {
        let bitrue_client = BitrueClient::new(
            get_config("BITRUE_API_KEY").unwrap(),
            get_config("BITRUE_SECRET_KEY").unwrap(),
        );
        let lbank_client = LBankClient::new(
            get_config("LBANK_API_KEY").unwrap(),
            get_config("LBANK_SECRET_KEY").unwrap(),
        );
        let order_executor = OrderExecutorBuilder::new()
            .with_exchange(Exchange::Bitrue, bitrue_client)
            .with_exchange(Exchange::LBank, lbank_client)
            .build();

        order_executor.run(order_rx, shutdown.clone());
    }

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

fn is_enabled(name: &str) -> bool {
    get_config(&format!("{name}_ENABLED")).unwrap_or_default()
}

fn get_config<T: FromStr>(key: &str) -> Option<T> {
    let value = dotenv::var(key).ok()?;
    Some(T::from_str(&value).unwrap_or_else(|_| panic!("Failed to read config value: {key}")))
}
