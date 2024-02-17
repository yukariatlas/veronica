use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RawData {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub spread: f64,
    pub date: NaiveDate,
    pub trading_volume: u64,
    pub trading_money: u64,
}

impl From<(f64, f64, f64, f64, f64, NaiveDate, u64, u64)> for RawData {
    fn from(
        (open, high, low, close, spread, date, trading_volume, trading_money): (
            f64,
            f64,
            f64,
            f64,
            f64,
            NaiveDate,
            u64,
            u64,
        ),
    ) -> Self {
        Self {
            open: open,
            high: high,
            low: low,
            close: close,
            spread: spread,
            date: date,
            trading_volume: trading_volume,
            trading_money: trading_money,
        }
    }
}

impl std::fmt::Display for RawData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "open:{}, high:{}, low:{}, close:{}, spread:{}, date:{}, trading volume:{}, trading money:{}",
        self.open, self.high, self.low, self.close, self.spread, self.date, self.trading_volume, self.trading_money)
    }
}

impl std::default::Default for RawData {
    fn default() -> Self {
        RawData {
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            spread: 0.0,
            date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            trading_volume: 0,
            trading_money: 0,
        }
    }
}
