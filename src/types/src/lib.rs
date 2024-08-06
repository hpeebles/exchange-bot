use async_trait::async_trait;
use std::collections::BTreeMap;
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
pub struct PendingMarketOrder {
    pub exchange: Exchange,
    pub amount: Amount8Decimals,
    pub expected_return: Amount8Decimals,
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
        let (whole, fractional) = s.split_once('.').unwrap_or((s, ""));
        let fractional_padded = format!("{fractional:0<4}");

        let units =
            (1_0000 * u128::from_str(whole).unwrap()) + u128::from_str(&fractional_padded).unwrap();

        Ok(Price4Decimals { units })
    }
}

impl FromStr for Amount8Decimals {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (whole, fractional) = s.split_once('.').unwrap_or((s, ""));
        let fractional_padded = format!("{fractional:0<8}");

        let units = (1_0000_0000 * u128::from_str(whole).unwrap())
            + u128::from_str(&fractional_padded).unwrap();

        Ok(Amount8Decimals { units })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("0.1234", 1234)]
    #[test_case("1.234", 12340)]
    #[test_case("123.4567", 1234567)]
    #[test_case("1234", 12340000)]
    fn parse_price(s: &str, expected_units: u128) {
        let result = Price4Decimals::from_str(s).unwrap();
        assert_eq!(result.units, expected_units);
    }

    #[test_case("0.1234", 12340000)]
    #[test_case("1.234", 123400000)]
    #[test_case("123.45678901", 12345678901)]
    #[test_case("1234", 123400000000)]
    fn parse_amount(s: &str, expected_units: u128) {
        let result = Amount8Decimals::from_str(s).unwrap();
        assert_eq!(result.units, expected_units);
    }
}
