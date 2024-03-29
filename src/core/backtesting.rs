use std::collections::HashMap;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::config::config;
use crate::crawler::crawler;
use crate::export::export;
use crate::storage::backend;
use crate::strategy::{schema, strategy};

use super::decision;

pub const PORTFOLIO_FILENAME: &str = "portfolio.yaml";
pub const FUND_DIAGRAM_FILENAME: &str = "fund_diagram.html";

#[derive(Serialize, Deserialize)]
pub struct StockTradeInfo {
    pub data_series: Vec<schema::RawData>,
    pub trade_series: Vec<(chrono::NaiveDate, chrono::NaiveDate)>,
}

pub struct Backtesting {
    pub config: config::Config,
    pub crawler: Rc<dyn crawler::Crawler>,
    pub backend_op: Rc<dyn backend::BackendOp>,
    pub strategy: strategy::Strategies,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub liquidity: u32,
    pub stocks_hold_num: usize,
    pub portfolios: Vec<decision::Portfolio>,
}

impl Backtesting {
    pub fn new(
        config: config::Config,
        crawler: Rc<dyn crawler::Crawler>,
        backend_op: Rc<dyn backend::BackendOp>,
        strategy: strategy::Strategies,
    ) -> Self {
        Backtesting {
            config,
            crawler,
            backend_op,
            strategy,
            start_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            liquidity: 200000,
            stocks_hold_num: 5,
            portfolios: Vec::new(),
        }
    }

    pub fn run(&mut self, start_date: chrono::NaiveDate, end_date: chrono::NaiveDate) {
        self.start_date = start_date;
        self.end_date = end_date;

        let strategy = Rc::new(strategy::StrategyFactory::get(
            self.strategy.clone(),
            self.backend_op.clone(),
        ));
        let mut decision =
            decision::Decision::new(self.crawler.clone(), self.backend_op.clone(), strategy);
        let mut date = self.start_date;
        let mut stocks_hold = HashMap::new();
        let mut trade_stocks = HashMap::new();

        decision.liquidity = self.liquidity;
        decision.stocks_hold_num = self.stocks_hold_num;

        while date <= self.end_date {
            let portfolio_opt = decision.calc_portfolio(date).unwrap();

            if portfolio_opt.is_some() {
                let portfolio = portfolio_opt.unwrap();

                for stock_info in &portfolio.stocks_settled {
                    let hold_date = stocks_hold.get(&stock_info.stock_id).unwrap();

                    trade_stocks
                        .entry(stock_info.stock_id.to_owned())
                        .or_insert(Vec::new())
                        .push((*hold_date, date));
                    stocks_hold.remove(&stock_info.stock_id);
                }
                for stock_info in &portfolio.stocks_selected {
                    stocks_hold.insert(stock_info.stock_id.to_owned(), date);
                }
                self.portfolios.push(portfolio);
            }
            date = date.succ_opt().unwrap();
        }

        self.export_trade(&trade_stocks);
        self.draw_diagram(&trade_stocks);
    }

    fn get_full_path(&self, filename: &str) -> String {
        self.config.portfolio_path.to_owned() + "/" + filename
    }

    fn get_stock_trade_info(
        &self,
        stock_id: &str,
        trade_series: &Vec<(chrono::NaiveDate, chrono::NaiveDate)>,
    ) -> StockTradeInfo {
        let records = self
            .backend_op
            .query_by_range(&stock_id, self.start_date, self.end_date)
            .unwrap();

        StockTradeInfo {
            data_series: records,
            trade_series: trade_series.to_vec(),
        }
    }

    fn export_trade(
        &self,
        trade_stocks: &HashMap<String, Vec<(chrono::NaiveDate, chrono::NaiveDate)>>,
    ) {
        std::fs::create_dir_all(&self.config.portfolio_path).unwrap();

        for (stock_id, trade_series) in trade_stocks {
            export::to_yaml(
                &self.get_full_path(&(stock_id.to_owned() + ".yaml")),
                &self.get_stock_trade_info(&stock_id, &trade_series),
            );
        }
        export::to_yaml(&self.get_full_path(PORTFOLIO_FILENAME), &self.portfolios);
    }

    fn draw_diagram(
        &self,
        trade_stocks: &HashMap<String, Vec<(chrono::NaiveDate, chrono::NaiveDate)>>,
    ) {
        std::fs::create_dir_all(&self.config.portfolio_path).unwrap();

        for (stock_id, trade_series) in trade_stocks {
            self.draw_trade_diagram(
                &stock_id,
                &self.get_stock_trade_info(&stock_id, &trade_series),
            );
        }
        self.draw_fund_diagram();
    }

    fn draw_trade_diagram(&self, stock_id: &str, trade_info: &StockTradeInfo) {
        let mut plot = plotly::Plot::new();
        let mut layout = plotly::Layout::new();
        let mut date_series = Vec::new();
        let mut open_series = Vec::new();
        let mut high_series = Vec::new();
        let mut low_series = Vec::new();
        let mut close_series = Vec::new();

        for record in &trade_info.data_series {
            date_series.push(record.date.to_string());
            open_series.push(record.open);
            high_series.push(record.high);
            low_series.push(record.low);
            close_series.push(record.close);
        }

        for (hold_date, settle_date) in &trade_info.trade_series {
            layout.add_shape(
                plotly::layout::Shape::new()
                    .x_ref("x")
                    .y_ref("paper")
                    .shape_type(plotly::layout::ShapeType::Rect)
                    .x0(hold_date.to_string())
                    .y0(0)
                    .x1(settle_date.to_string())
                    .y1(1)
                    .fill_color(plotly::common::color::NamedColor::BurlyWood)
                    .opacity(0.5)
                    .layer(plotly::layout::ShapeLayer::Below)
                    .line(plotly::layout::ShapeLine::new().width(0.)),
            );
        }

        let trace = Box::new(
            plotly::Candlestick::new(
                date_series.clone(),
                open_series.clone(),
                high_series.clone(),
                low_series.clone(),
                close_series.clone(),
            )
            .name(&stock_id),
        );

        plot.add_trace(trace);
        plot.set_layout(layout);
        plot.write_html(self.get_full_path(&(stock_id.to_owned() + ".html")));
    }

    fn draw_fund_diagram(&self) {
        let mut plot = plotly::Plot::new();
        let mut date_series = Vec::new();
        let mut fund_series = Vec::new();
        let mut text_series = Vec::new();

        for portfolio in &self.portfolios {
            let mut fund = portfolio.liquidity;

            for stock_info in &portfolio.stocks_hold {
                fund += stock_info.price * stock_info.num;
            }
            for stock_info in &portfolio.stocks_selected {
                fund += stock_info.price * stock_info.num;
            }
            date_series.push(portfolio.date);
            fund_series.push(fund);
            text_series.push(portfolio.to_string());
        }

        let trace = plotly::Scatter::new(date_series, fund_series)
            .text_array(text_series)
            .mode(plotly::common::Mode::Lines)
            .name("Fund");

        plot.add_trace(trace);
        plot.write_html(self.get_full_path(FUND_DIAGRAM_FILENAME));
    }
}
