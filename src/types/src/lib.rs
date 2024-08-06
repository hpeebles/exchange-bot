use async_trait::async_trait;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum Exchange {
    LBank,
    Bitrue,
}

#[async_trait]
pub trait ExchangeSubscriber {
    async fn run_async(self, sender: Sender<OrderbookState>, cancellation_token: CancellationToken);
}

#[derive(Debug)]
pub struct OrderbookState {
    pub timestamp_ms: u64,
    pub asks: BTreeMap<Price4Decimals, Amount8Decimals>,
    pub bids: BTreeMap<Price4Decimals, Amount8Decimals>,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Price4Decimals {
    units: u128,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Amount8Decimals {
    units: u128,
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
        let result = Price4Decimals::from_str(&s).unwrap();
        assert_eq!(result.units, expected_units);
    }
}
