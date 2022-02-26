use std::result::Result;
use chrono::NaiveDate;
use serde::{Serialize, Deserialize};
use ta::indicators::{SimpleMovingAverage, StandardDeviation};
use ta::Next;

use crate::strategy::{schema, bollinger_band};

pub enum Views {
    None,
    BollingerBand,
}

#[derive(Debug)]
pub enum Error {
    Ta(ta::errors::TaError),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BollingerBandView {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub date: NaiveDate,
    pub volume: u64,
    pub sma: f64,
    pub sd: f64,
}

pub trait Transform {
    type View;
    
    fn transform(records: &Vec<schema::RawData>) -> Result<Vec<Self::View>, Error>;
}

impl From<ta::errors::TaError> for Error {
    fn from(err: ta::errors::TaError) -> Error {
        Error::Ta(err)
    }
}

impl Default for BollingerBandView {
    fn default() -> BollingerBandView {
        BollingerBandView {
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            date: chrono::NaiveDate::from_ymd(1970, 1, 1),
            volume: 0,
            sma: 0.0,
            sd: 0.0,
        }
    }
}

impl Transform for BollingerBandView {
    type View = BollingerBandView;

    fn transform(records: &Vec<schema::RawData>) -> Result<Vec<Self::View>, Error> {
        let mut views = Vec::new();
        let mut sd = StandardDeviation::new(bollinger_band::PERIOD)?;
        let mut sma = SimpleMovingAverage::new(bollinger_band::PERIOD)?;
        
        for (idx, record) in records.iter().enumerate() {
            let mut view = BollingerBandView {
                open: record.open,
                high: record.high,
                low: record.low,
                close: record.close,
                date: record.date,
                volume: record.trading_volume,
                ..Default::default()
            };
            view.sma = sma.next((record.high + record.low + record.close) / 3.0);
            view.sd = sd.next((record.high + record.low + record.close) / 3.0);
            
            if idx + 1 >= bollinger_band::PERIOD {
                views.push(view);
            }
        }
        
        Ok(views)
    }
}