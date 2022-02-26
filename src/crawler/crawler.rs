use std::{result::Result, io::Read};
use chrono::NaiveDate;
use mockall::automock;
use crate::strategy::schema;

const STOCK_MONTH_REVENUE_URL: &str = "https://quality.data.gov.tw/dq_download_csv.php?nid=11549&md5_url=da96048521360db9f23a2b47c9c31155";

pub struct Args {
    pub stock_id: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Url(url::ParseError),
    Io(std::io::Error),
    Csv(csv::Error),
    BadRequest,
    RateLimitReached,
    Unknown,
}

#[automock]
pub trait Crawler {
    fn get_stock_data(&self, args: &Args) -> Result<Vec<schema::RawData>, Error>;
    fn get_stock_list(&self) -> Result<Vec<String>, Error> {
        let mut resp = reqwest::blocking::get(STOCK_MONTH_REVENUE_URL)?;
        let mut buf = Vec::new();
        let mut stock_list = Vec::new();
        
        resp.read_to_end(&mut buf)?;
        for result in csv::Reader::from_reader(&*buf).records() {
            let record = result?;
            stock_list.push(record[0].to_owned());
        }

        Ok(stock_list)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::Reqwest(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Error {
        Error::Url(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<csv::Error> for Error {
    fn from(err: csv::Error) -> Error {
        Error::Csv(err)
    }
}