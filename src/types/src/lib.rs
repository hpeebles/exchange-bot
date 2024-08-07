use async_trait::async_trait;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Write};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Exchange {
    LBank,
    Bitrue,
}

#[async_trait]
pub trait ExchangeSubscriber {
    async fn run_async(
        self,
        sender: Sender<Arc<OrderbookState>>,
        cancellation_token: CancellationToken,
    );
}

#[async_trait]
pub trait ExchangeOrderExecutor: Send {
    async fn submit_order(&self, order: PendingOrder) -> Result<String, String>;
}

pub trait OrderbookStateProcessor {
    fn run(self, updates: Receiver<Arc<OrderbookState>>, cancellation_token: CancellationToken);
}

#[derive(Clone, Debug)]
pub struct OrderbookState {
    pub exchange: Exchange,
    pub timestamp_ms: u64,
    pub asks: BTreeMap<Price4Decimals, Amount8Decimals>,
    pub bids: BTreeMap<Price4Decimals, Amount8Decimals>,
}

impl OrderbookState {
    pub fn best_bid(&self) -> Option<Order> {
        self.bids.iter().next_back().map(|(p, a)| Order {
            exchange: self.exchange,
            price: *p,
            amount: *a,
        })
    }

    pub fn best_ask(&self) -> Option<Order> {
        self.asks.iter().next().map(|(p, a)| Order {
            exchange: self.exchange,
            price: *p,
            amount: *a,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ArbOpportunity {
    pub buy: Order,
    pub sell: Order,
}

#[derive(Clone, Debug)]
pub struct Order {
    pub exchange: Exchange,
    pub price: Price4Decimals,
    pub amount: Amount8Decimals,
}

#[derive(Clone, Debug)]
pub enum PendingOrder {
    Limit(PendingLimitOrder),
    Market(PendingMarketOrder),
}

impl PendingOrder {
    pub fn exchange(&self) -> Exchange {
        match self {
            PendingOrder::Limit(o) => o.exchange,
            PendingOrder::Market(o) => o.exchange,
        }
    }

    pub fn direction(&self) -> Direction {
        match self {
            PendingOrder::Limit(o) => o.direction,
            PendingOrder::Market(o) => o.direction,
        }
    }

    pub fn amount(&self) -> Amount8Decimals {
        match self {
            PendingOrder::Limit(o) => o.amount,
            PendingOrder::Market(o) => o.amount,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PendingLimitOrder {
    pub exchange: Exchange,
    pub direction: Direction,
    pub amount: Amount8Decimals,
    pub price: Price4Decimals,
}

#[derive(Clone, Debug)]
pub struct PendingMarketOrder {
    pub exchange: Exchange,
    pub direction: Direction,
    pub amount: Amount8Decimals,
    pub expected_return: Amount8Decimals,
}

#[derive(Copy, Clone, Debug)]
pub enum Direction {
    Buy,
    Sell,
}

impl Direction {
    pub fn is_buy(&self) -> bool {
        matches!(self, Direction::Buy)
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Price4Decimals {
    units: u128,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Amount8Decimals {
    units: u128,
}

impl Price4Decimals {
    pub fn from_units(units: u128) -> Price4Decimals {
        Price4Decimals { units }
    }

    pub fn units(&self) -> u128 {
        self.units
    }
}

impl Amount8Decimals {
    pub fn from_units(units: u128) -> Amount8Decimals {
        Amount8Decimals { units }
    }

    pub fn from_whole(value: u128) -> Amount8Decimals {
        Amount8Decimals {
            units: value * 1_0000_0000,
        }
    }

    pub fn units(&self) -> u128 {
        self.units
    }
}

impl FromStr for Price4Decimals {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let units = parse_units_from_decimal(s, 4)?;

        Ok(Price4Decimals { units })
    }
}

impl FromStr for Amount8Decimals {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let units = parse_units_from_decimal(s, 8)?;

        Ok(Amount8Decimals { units })
    }
}

impl Display for Price4Decimals {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        format_units_to_decimal(self.units, 4, f)
    }
}

impl Display for Amount8Decimals {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        format_units_to_decimal(self.units, 8, f)
    }
}

fn parse_units_from_decimal(s: &str, decimals: usize) -> Result<u128, ()> {
    let (whole, fractional) = s.split_once('.').unwrap_or((s, ""));
    let fractional_padded = format!("{fractional:0<decimals$}");
    let units_per_whole = 10u128.pow(decimals as u32);

    let units = (units_per_whole * u128::from_str(whole).unwrap())
        + u128::from_str(&fractional_padded).unwrap();

    Ok(units)
}

fn format_units_to_decimal(
    units: u128,
    decimals: usize,
    f: &mut Formatter<'_>,
) -> std::fmt::Result {
    let units_per_whole = 10u128.pow(decimals as u32);
    let fractional = units % units_per_whole;
    let whole = units / units_per_whole;

    Display::fmt(&whole, f)?;

    if fractional != 0 {
        f.write_char('.')?;
        let fractional_padded = format!("{fractional:0<decimals$}");
        let fractional_trimmed = fractional_padded.trim_end_matches('0');
        f.write_str(fractional_trimmed)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("0.1234", 1234)]
    #[test_case("1.234", 12340)]
    #[test_case("123.4567", 1234567)]
    #[test_case("1234", 12340000)]
    fn price_roundtrip(s: &str, expected_units: u128) {
        let result = Price4Decimals::from_str(s).unwrap();
        assert_eq!(result.units, expected_units);

        let fmt = result.to_string();
        assert_eq!(fmt.as_str(), s);
    }

    #[test_case("0.1234", 12340000)]
    #[test_case("1.234", 123400000)]
    #[test_case("123.45678901", 12345678901)]
    #[test_case("1234", 123400000000)]
    fn amount_roundtrip(s: &str, expected_units: u128) {
        let result = Amount8Decimals::from_str(s).unwrap();
        assert_eq!(result.units, expected_units);

        let fmt = result.to_string();
        assert_eq!(fmt.as_str(), s);
    }
}
