use std::result::Result;
use serde::Deserialize;
use crate::crawler::crawler;
use crate::strategy::schema;

const FINMIND_V4_URL: &str = "https://api.finmindtrade.com/api/v4/data";
const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaiwanStockPrice {
    pub stock_id: String,
    pub open: f64,
    pub max: f64,
    pub min: f64,
    pub close: f64,
    pub spread: f64,
    pub date: chrono::NaiveDate,
    #[serde(alias = "Trading_Volume")]
    pub trading_volume: u64,
    #[serde(alias = "Trading_money")]
    pub trading_money: u64,
    #[serde(alias = "Trading_turnover")]
    pub trading_turnover: f64,
}

impl From<TaiwanStockPrice> for schema::RawData {
    fn from(record: TaiwanStockPrice) -> schema::RawData {
        schema::RawData {
            open: record.open,
            high: record.max,
            low: record.min,
            close: record.close,
            spread: record.spread,
            date: record.date,
            trading_volume: record.trading_volume,
            trading_money: record.trading_money
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Response {
    pub msg: String,
    pub status: usize,
    pub data: Vec<TaiwanStockPrice>,
}

pub struct Finmind {
    token: String,
}

impl Finmind {
    pub fn new(token: &str) -> Self {
        Finmind {
            token: token.to_owned()
        }
    }
}

impl crawler::Crawler for Finmind {
    fn get_stock_data(&self, args: &crawler::Args) -> Result<Vec<schema::RawData>, crawler::Error> {
        let url = reqwest::Url::parse_with_params(
            FINMIND_V4_URL,
            &[
                ("data_id", args.stock_id.to_owned()),
                ("dataset", "TaiwanStockPrice".to_owned()),
                (
                    "start_date",
                    args.start_date.format(DEFAULT_DATE_FORMAT).to_string(),
                ),
                (
                    "end_date",
                    args.end_date.format(DEFAULT_DATE_FORMAT).to_string(),
                ),
                ("token", self.token.to_owned()),
                ],
            )?;
            
        let resp: Response = reqwest::blocking::get(url)?.json()?;

        match resp.status {
            200 => Ok(resp.data.into_iter().map(|record| record.into()).collect()),
            400 => Err(crawler::Error::BadRequest),
            402 => Err(crawler::Error::RateLimitReached),
            _ => Err(crawler::Error::Unknown),
        }
    }
}