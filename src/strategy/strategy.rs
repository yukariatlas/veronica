use std::cmp::Ordering;
use std::rc::Rc;
use std::result::Result;

use crate::dataview::view;
use crate::storage::backend;

use super::bollinger_band;

#[derive(Clone)]
pub enum Strategies {
    BollingerBand,
}

#[derive(Debug, Clone, Eq)]
pub struct Score {
    pub point: i64,
    pub trading_volume: u64,
}

impl std::default::Default for Score {
    fn default() -> Self {
        Score {
            point: 0,
            trading_volume: 0,
        }
    }
}

impl PartialEq for Score {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.point == other.point && self.trading_volume == other.trading_volume
    }
}

impl PartialOrd for Score {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.point != other.point {
            return self.point.partial_cmp(&other.point);
        }
        self.trading_volume.partial_cmp(&other.trading_volume)
    }
}

impl Ord for Score {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        if self.point != other.point {
            return self.point.cmp(&other.point);
        }
        self.trading_volume.cmp(&other.trading_volume)
    }
}

#[derive(Debug)]
pub enum Error {
    Backend(backend::Error),
    Dataview(view::Error),
    BadOperation,
    RecordNotFound,
}

impl From<backend::Error> for Error {
    fn from(err: backend::Error) -> Error {
        Error::Backend(err)
    }
}

impl From<view::Error> for Error {
    fn from(err: view::Error) -> Error {
        Error::Dataview(err)
    }
}

pub enum Strategy {
    BollingerBand(bollinger_band::Strategy),
}

#[mockall::automock]
pub trait StrategyAPI {
    fn analyze(&self, stock_id: &str, assess_date: chrono::NaiveDate) -> Result<Score, Error>;
    fn settle_check(
        &self,
        stock_id: &str,
        hold_date: chrono::NaiveDate,
        assess_date: chrono::NaiveDate,
    ) -> Result<bool, Error>;
    fn draw_view(&self, stock_id: &str) -> Result<(), Error>;
}

impl StrategyAPI for Strategy {
    fn analyze(&self, stock_id: &str, assess_date: chrono::NaiveDate) -> Result<Score, Error> {
        match *self {
            Strategy::BollingerBand(ref bollinger_band) => {
                bollinger_band.analyze(stock_id, assess_date)
            }
        }
    }
    fn settle_check(
        &self,
        stock_id: &str,
        hold_date: chrono::NaiveDate,
        assess_date: chrono::NaiveDate,
    ) -> Result<bool, Error> {
        match *self {
            Strategy::BollingerBand(ref bollinger_band) => {
                bollinger_band.settle_check(stock_id, hold_date, assess_date)
            }
        }
    }
    fn draw_view(&self, stock_id: &str) -> Result<(), Error> {
        match *self {
            Strategy::BollingerBand(ref bollinger_band) => bollinger_band.draw_view(stock_id),
        }
    }
}

pub struct StrategyFactory {}

impl StrategyFactory {
    pub fn get(strategy: Strategies, backend_op: Rc<dyn backend::BackendOp>) -> Strategy {
        match strategy {
            Strategies::BollingerBand => Strategy::BollingerBand(bollinger_band::Strategy {
                backend_op: backend_op,
            }),
        }
    }
}
