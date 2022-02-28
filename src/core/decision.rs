use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

use serde::{Serialize, Deserialize};

use crate::crawler::crawler;
use crate::strategy::schema;
use crate::strategy::strategy;
use crate::storage::backend;

#[derive(Debug)]
pub enum Error {
    Backend(backend::Error),
    Crawler(crawler::Error),
    Strategy(strategy::Error),
    BackendRecordNotFound,
}

impl From<backend::Error> for Error {
    fn from(err: backend::Error) -> Error {
        Error::Backend(err)
    }
}

impl From<crawler::Error> for Error {
    fn from(err: crawler::Error) -> Error {
        Error::Crawler(err)
    }
}

impl From<strategy::Error> for Error {
    fn from(err: strategy::Error) -> Error {
        Error::Strategy(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StockInfo {
    pub stock_id: String,
    pub num: u32,
    pub price: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Portfolio {
    pub date: chrono::NaiveDate,
    pub stocks_selected: Vec<StockInfo>,
    pub stocks_hold: Vec<StockInfo>,
    pub stocks_settled: Vec<StockInfo>,
    pub liquidity: u32,
}

impl std::default::Default for Portfolio {
    fn default() -> Self {
        Portfolio {
            date: chrono::NaiveDate::from_ymd(1970, 1, 1),
            stocks_selected: Vec::new(),
            stocks_hold: Vec::new(),
            stocks_settled: Vec::new(),
            liquidity: 0
        }
    }
}

impl std::fmt::Display for Portfolio {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut stock_ids = Vec::new();

        stock_ids.extend(self.stocks_selected.iter().map(|stock_info| stock_info.stock_id.to_owned()));
        stock_ids.extend(self.stocks_hold.iter().map(|stock_info| stock_info.stock_id.to_owned()));

        fmt.write_str("Stocks: ")?;
        fmt.write_str(&stock_ids.join(", "))?;
        Ok(())
    }
}

pub struct Decision {
    pub crawler: Rc<dyn crawler::Crawler>,
    pub backend_op: Rc<dyn backend::BackendOp>,
    pub strategy: Rc<dyn strategy::StrategyAPI>,
    pub stocks_hold_num: usize,
    pub liquidity: u32,
    stocks_hold: HashMap<String, (chrono::NaiveDate, u32)>,
}

impl Decision {
    pub fn new(crawler: Rc<dyn crawler::Crawler>, backend_op: Rc<dyn backend::BackendOp>, strategy: Rc<dyn strategy::StrategyAPI>) -> Self {
        Decision {
            crawler: crawler,
            backend_op: backend_op,
            strategy: strategy,
            stocks_hold_num: 5,
            liquidity: 200000,
            stocks_hold: HashMap::new()
        }
    }
    fn get_select_stocks(&self, assess_date: chrono::NaiveDate) -> Result<Vec<String>, Error> {
        let stock_list = self.crawler.get_stock_list().unwrap_or(vec![]);
        let mut stock_scores: Vec<(String, strategy::Score)> = Vec::new();
        let mut stocks_selected = Vec::new();

        for stock_id in stock_list {
            stock_scores.push((stock_id.clone(), self.strategy.analyze(&stock_id, assess_date)?));
        }
    
        stock_scores.sort_by(|lhs, rhs| rhs.1.cmp(&lhs.1));
    
        for (stock_id, score) in stock_scores.iter() {
            if self.stocks_hold.len() + stocks_selected.len() == self.stocks_hold_num {
                break;
            }
            if score.point <= 0 {
                break;
            }
            if self.stocks_hold.iter().position(|(_stock_id, _)| _stock_id == stock_id).is_none() {
                stocks_selected.push(stock_id.to_owned());
            }
        }

        Ok(stocks_selected)
    }

    fn get_settle_stocks(&self, assess_date: chrono::NaiveDate) -> Result<Vec<String>, Error> {
        let mut stocks_settled = Vec::new();

        for (stock_id, (hold_date, _)) in &self.stocks_hold {
            if self.strategy.settle_check(stock_id, *hold_date, assess_date)? {
                stocks_settled.push(stock_id.to_owned());
            }
        }

        Ok(stocks_settled)
    }

    fn handle_settle_stocks(&mut self, assess_date: chrono::NaiveDate, portfolio: &mut Portfolio) -> Result<(), Error> {
        for stock_id in self.get_settle_stocks(assess_date)? {
            let stock_num = self.stocks_hold.get(&stock_id).ok_or(Error::BackendRecordNotFound)?.1;
            let record = self.backend_op.query(&stock_id, assess_date)?.ok_or(Error::BackendRecordNotFound)?;
            let price = ((record.high + record.low) / 2.0) as u32;

            portfolio.stocks_settled.push(StockInfo {
                stock_id: stock_id.to_owned(),
                num: stock_num,
                price: price,
            });
            self.liquidity += stock_num * price;
            self.stocks_hold.remove(&stock_id);
        }

        portfolio.liquidity = self.liquidity;
        Ok(())
    }

    fn handle_hold_stocks(&mut self, assess_date: chrono::NaiveDate, portfolio: &mut Portfolio) -> Result<(), Error> {
        for stock_id in self.stocks_hold.keys().cloned() {
            let mut data = self.backend_op.query(&stock_id, assess_date)?;
            let record = data.get_or_insert(schema::RawData::default());

            portfolio.stocks_hold.push(StockInfo {
                stock_id: stock_id.to_owned(),
                num: self.stocks_hold.get(&stock_id).ok_or(Error::BackendRecordNotFound)?.1,
                price: ((record.high + record.low) / 2.0) as u32,
            });
        }

        portfolio.liquidity = self.liquidity;
        Ok(())
    }

    fn handle_selected_stocks(&mut self, assess_date: chrono::NaiveDate, portfolio: &mut Portfolio) -> Result<(), Error> {
        let stocks_selected = self.get_select_stocks(assess_date)?;

        if !stocks_selected.is_empty() {
            let invest_max_per_stock = self.liquidity / stocks_selected.len() as u32;

            for stock_id in stocks_selected {
                let record = self.backend_op.query(&stock_id, assess_date)?.ok_or(Error::BackendRecordNotFound)?;
                let price = ((record.high + record.low) / 2.0) as u32;
                let stock_num = invest_max_per_stock / price;

                portfolio.stocks_selected.push(StockInfo {
                    stock_id: stock_id.to_owned(),
                    num: stock_num,
                    price: price,
                });
                self.liquidity -= stock_num * price;
                self.stocks_hold.insert(stock_id, (assess_date, stock_num));
            }
        }

        portfolio.liquidity = self.liquidity;
        Ok(())
    }

    fn has_trading_data(&self, assess_date: chrono::NaiveDate) -> Result<bool, Error> {
        for stock_id in self.stocks_hold.keys().cloned() {
            if self.backend_op.query(&stock_id, assess_date)?.is_none() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn calc_portfolio(&mut self, assess_date: chrono::NaiveDate) -> Result<Option<Portfolio>, Error> {
        if !self.has_trading_data(assess_date)? {
            return Ok(None);
        }

        let mut portfolio = Portfolio {
            date: assess_date,
            stocks_selected: Vec::new(),
            stocks_hold: Vec::new(),
            stocks_settled: Vec::new(),
            liquidity: 0,
        };

        self.handle_settle_stocks(assess_date, &mut portfolio)?;
        self.handle_hold_stocks(assess_date, &mut portfolio)?;
        self.handle_selected_stocks(assess_date, &mut portfolio)?;
        Ok(Some(portfolio))
    }
}

#[cfg(test)]
mod decision_test {
    use std::rc::Rc;

    use crate::core::decision::Decision;
    use crate::crawler::crawler;
    use crate::storage::backend;
    use crate::strategy::{strategy, schema};

    #[test]
    fn select_stocks_all_zero_score() {
        let mut mock_crawler = crawler::MockCrawler::new();
        let mut mock_backend_op = backend::MockBackendOp::new();
        let mut mock_strategy = strategy::MockStrategyAPI::new();

        mock_crawler.expect_get_stock_list()
            .returning(|| {
                Ok(vec!["0050".to_owned(), "0051".to_owned(), "0052".to_owned()])
            });
        mock_backend_op.expect_query()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(Some(schema::RawData {
                        ..Default::default()
                    })),
                    "0051" => return Ok(Some(schema::RawData {
                        ..Default::default()
                    })),
                    "0052" => return Ok(Some(schema::RawData {
                        ..Default::default()
                    })),
                    _ => return Ok(None),
                }
            });
        mock_strategy.expect_analyze()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(strategy::Score {
                        point: 0,
                        trading_volume: 0
                    }),
                    "0051" => return Ok(strategy::Score {
                        point: 0,
                        trading_volume: 0
                    }),
                    "0052" => return Ok(strategy::Score {
                        point: 0,
                        trading_volume: 0
                    }),
                    _ => return Ok(strategy::Score::default()),
                }
            });

        let mut decision = Decision::new(Rc::new(mock_crawler), Rc::new(mock_backend_op), Rc::new(mock_strategy));
        let portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 1)).unwrap().unwrap();

        assert!(portfolio.stocks_selected.is_empty());
    }

    #[test]
    fn select_stocks_score_in_order() {
        let mut mock_crawler = crawler::MockCrawler::new();
        let mut mock_backend_op = backend::MockBackendOp::new();
        let mut mock_strategy = strategy::MockStrategyAPI::new();

        mock_crawler.expect_get_stock_list()
            .returning(|| {
                Ok(vec!["0050".to_owned(), "0051".to_owned(), "0052".to_owned()])
            });
        mock_backend_op.expect_query()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(Some(schema::RawData {
                        low: 1.0,
                        high: 1.0,
                        ..Default::default()
                    })),
                    "0051" => return Ok(Some(schema::RawData {
                        low: 1.0,
                        high: 1.0,
                        ..Default::default()
                    })),
                    "0052" => return Ok(Some(schema::RawData {
                        high: 1.0,
                        low: 1.0,
                        ..Default::default()
                    })),
                    _ => return Ok(None),
                }
            });
        mock_strategy.expect_analyze()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(strategy::Score {
                        point: 2,
                        trading_volume: 0
                    }),
                    "0051" => return Ok(strategy::Score {
                        point: 3,
                        trading_volume: 0
                    }),
                    "0052" => return Ok(strategy::Score {
                        point: 4,
                        trading_volume: 0
                    }),
                    _ => return Ok(strategy::Score::default()),
                }
            });

        let expected_stock_ids = vec!["0052".to_owned(), "0051".to_owned(), "0050".to_owned()];
        let mut decision = Decision::new(Rc::new(mock_crawler), Rc::new(mock_backend_op), Rc::new(mock_strategy));
        let portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 1)).unwrap().unwrap();
        let selected_stock_ids: Vec<String> = portfolio.stocks_selected.into_iter().map(|stock_info| stock_info.stock_id).collect();

        assert_eq!(selected_stock_ids, expected_stock_ids);
    }

    #[test]
    fn select_stocks_score_no_duplicated_id() {
        let mut mock_crawler = crawler::MockCrawler::new();
        let mut mock_backend_op = backend::MockBackendOp::new();
        let mut mock_strategy = strategy::MockStrategyAPI::new();

        mock_crawler.expect_get_stock_list()
            .returning(|| {
                Ok(vec!["0050".to_owned(), "0051".to_owned(), "0052".to_owned()])
            });
        mock_backend_op.expect_query()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(Some(schema::RawData {
                        low: 1.0,
                        high: 1.0,
                        ..Default::default()
                    })),
                    "0051" => return Ok(Some(schema::RawData::default())),
                    "0052" => return Ok(Some(schema::RawData::default())),
                    _ => return Ok(None),
                }
            });
        mock_strategy.expect_analyze()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(strategy::Score {
                        point: 2,
                        trading_volume: 0
                    }),
                    "0051" => return Ok(strategy::Score::default()),
                    "0052" => return Ok(strategy::Score::default()),
                    _ => return Ok(strategy::Score::default()),
                }
            });
        mock_strategy.expect_settle_check()
            .returning(|_, _, _| {
                Ok(false)
            });

        let expected_stock_ids = vec!["0050".to_owned()];
        let mut decision = Decision::new(Rc::new(mock_crawler), Rc::new(mock_backend_op), Rc::new(mock_strategy));
        let mut selected_stock_ids: Vec<String> = Vec::new();

        decision.stocks_hold_num = 4;

        let mut portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 1)).unwrap().unwrap();

        for stock_info in portfolio.stocks_selected {
            selected_stock_ids.push(stock_info.stock_id);
        }
        portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 2)).unwrap().unwrap();
        for stock_info in portfolio.stocks_selected {
            selected_stock_ids.push(stock_info.stock_id);
        }

        assert_eq!(selected_stock_ids, expected_stock_ids);
    }

    #[test]
    fn select_stocks_num_check() {
        let mut mock_crawler = crawler::MockCrawler::new();
        let mut mock_backend_op = backend::MockBackendOp::new();
        let mut mock_strategy = strategy::MockStrategyAPI::new();

        mock_crawler.expect_get_stock_list()
            .returning(|| {
                Ok(vec!["0050".to_owned()])
            });
        mock_backend_op.expect_query()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(Some(schema::RawData {
                        low: 2.0,
                        high: 8.0,
                        ..Default::default()
                    })),
                    _ => return Ok(None),
                }
            });
        mock_strategy.expect_analyze()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(strategy::Score {
                        point: 1,
                        trading_volume: 0
                    }),
                    _ => return Ok(strategy::Score::default()),
                }
            });

        let mut decision = Decision::new(Rc::new(mock_crawler), Rc::new(mock_backend_op), Rc::new(mock_strategy));

        decision.liquidity = 8;

        let portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 1)).unwrap().unwrap();
    
        assert_eq!(portfolio.stocks_selected.len(), 1);
        assert_eq!(portfolio.stocks_selected[0].stock_id, "0050");
        assert_eq!(portfolio.stocks_selected[0].num, 1);
        assert_eq!(portfolio.stocks_selected[0].price, 5);
    }

    #[test]
    fn hold_stocks_detail_check() {
        let mut mock_crawler = crawler::MockCrawler::new();
        let mut mock_backend_op = backend::MockBackendOp::new();
        let mut mock_strategy = strategy::MockStrategyAPI::new();

        mock_crawler.expect_get_stock_list()
            .returning(|| {
                Ok(vec!["0050".to_owned()])
            });
        mock_backend_op.expect_query()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(Some(schema::RawData {
                        low: 2.0,
                        high: 8.0,
                        ..Default::default()
                    })),
                    _ => return Ok(None),
                }
            });
        mock_strategy.expect_analyze()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(strategy::Score {
                        point: 2,
                        trading_volume: 0
                    }),
                    _ => return Ok(strategy::Score::default()),
                }
            });
        mock_strategy.expect_settle_check()
            .returning(|_, _, _| {
                Ok(false)
            });

        let mut decision = Decision::new(Rc::new(mock_crawler), Rc::new(mock_backend_op), Rc::new(mock_strategy));

        decision.liquidity = 8;
        decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 1)).unwrap().unwrap();

        let portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 2)).unwrap().unwrap();

        assert_eq!(portfolio.stocks_selected.len(), 0);
        assert_eq!(portfolio.stocks_hold.len(), 1);
        assert_eq!(portfolio.stocks_settled.len(), 0);
        assert_eq!(portfolio.stocks_hold[0].stock_id, "0050");
        assert_eq!(portfolio.stocks_hold[0].num, 1);
        assert_eq!(portfolio.stocks_hold[0].price, 5);
    }

    #[test]
    fn settle_stocks_detail_check() {
        let mut mock_crawler = crawler::MockCrawler::new();
        let mut mock_backend_op = backend::MockBackendOp::new();
        let mut mock_strategy = strategy::MockStrategyAPI::new();

        mock_crawler.expect_get_stock_list()
            .returning(|| {
                Ok(vec!["0050".to_owned()])
            });
        mock_backend_op.expect_query()
            .returning(|stock_id, _| {
                match stock_id {
                    "0050" => return Ok(Some(schema::RawData {
                        low: 2.0,
                        high: 8.0,
                        ..Default::default()
                    })),
                    _ => return Ok(None),
                }
            });
        mock_strategy.expect_analyze()
            .returning(|stock_id, assess_date| {
                match stock_id {
                    "0050" => return Ok(strategy::Score {
                        point: (assess_date == chrono::NaiveDate::from_ymd(1970, 1, 1)) as i64,
                        trading_volume: 0
                    }),
                    _ => return Ok(strategy::Score::default()),
                }
            });
        mock_strategy.expect_settle_check()
            .returning(|_, _, _| {
                Ok(true)
            });

        let mut decision = Decision::new(Rc::new(mock_crawler), Rc::new(mock_backend_op), Rc::new(mock_strategy));

        decision.liquidity = 8;
        decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 1)).unwrap().unwrap();

        let portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 2)).unwrap().unwrap();

        assert_eq!(portfolio.stocks_selected.len(), 0);
        assert_eq!(portfolio.stocks_hold.len(), 0);
        assert_eq!(portfolio.stocks_settled.len(), 1);
        assert_eq!(portfolio.stocks_settled[0].stock_id, "0050");
        assert_eq!(portfolio.stocks_settled[0].num, 1);
        assert_eq!(portfolio.stocks_settled[0].price, 5);
    }

    #[test]
    fn liquidity_check() {
        let mut mock_crawler = crawler::MockCrawler::new();
        let mut mock_backend_op = backend::MockBackendOp::new();
        let mut mock_strategy = strategy::MockStrategyAPI::new();

        mock_crawler.expect_get_stock_list()
            .returning(|| {
                Ok(vec!["0050".to_owned(), "0051".to_owned()])
            });
        mock_backend_op.expect_query()
            .returning(|stock_id, date| {
                match stock_id {
                    "0050" => match &date.format("%Y-%m-%d").to_string()[..] {
                        "1970-01-01" => return Ok(Some(schema::RawData {
                            low: 2.0,
                            high: 8.0,
                            ..Default::default()
                        })),
                        "1970-01-02" => return Ok(Some(schema::RawData {
                            low: 4.0,
                            high: 16.0,
                            ..Default::default()
                        })),
                        _ => return Ok(None),
                    },
                    "0051" => match &date.format("%Y-%m-%d").to_string()[..] {
                        "1970-01-01" => return Ok(Some(schema::RawData {
                            low: 4.0,
                            high: 8.0,
                            ..Default::default()
                        })),
                        "1970-01-02" => return Ok(Some(schema::RawData {
                            low: 8.0,
                            high: 16.0,
                            ..Default::default()
                        })),
                        _ => return Ok(None),
                    }
                    _ => return Ok(None),
                }
            });
        mock_strategy.expect_analyze()
            .returning(|stock_id, assess_date| {
                match stock_id {
                    "0050" => match &assess_date.format("%Y-%m-%d").to_string()[..] {
                        "1970-01-01" => return Ok(strategy::Score {
                            point: 2,
                            trading_volume: 10,
                        }),
                        "1970-01-02" => return Ok(strategy::Score {
                            point: 0,
                            trading_volume: 0,
                        }),
                        _ => return Ok(strategy::Score::default()),
                    },
                    "0051" => match &assess_date.format("%Y-%m-%d").to_string()[..] {
                        "1970-01-01" => return Ok(strategy::Score {
                            point: 4,
                            trading_volume: 20,
                        }),
                        "1970-01-02" => return Ok(strategy::Score {
                            point: 0,
                            trading_volume: 0,
                        }),
                        _ => return Ok(strategy::Score::default()),
                    }
                    _ => return Ok(strategy::Score::default()),
                }
            });
        mock_strategy.expect_settle_check()
            .returning(|_, _, _| {
                Ok(true)
            });

        let mut decision = Decision::new(Rc::new(mock_crawler), Rc::new(mock_backend_op), Rc::new(mock_strategy));

        decision.liquidity = 20;

        let mut portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 1)).unwrap().unwrap();

        assert_eq!(portfolio.liquidity, 4);

        portfolio = decision.calc_portfolio(chrono::NaiveDate::from_ymd(1970, 1, 2)).unwrap().unwrap();
        assert_eq!(portfolio.liquidity, 36);
    }
}