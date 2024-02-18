use std::rc::Rc;

use crate::dataview::view::{self, Transform};
use crate::storage::backend;
use crate::strategy::strategy;

pub const PERIOD: usize = 30;
pub const ANALYZE_RANGE: usize = 8;
pub const BAND_SIZE: usize = 2;

pub struct Strategy {
    pub backend_op: Rc<dyn backend::BackendOp>,
}

impl Strategy {
    fn get_views(
        &self,
        stock_id: &str,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<view::BollingerBandView>, strategy::Error> {
        let calc_date = start_date
            .checked_sub_signed(chrono::Duration::days(PERIOD as i64 * 2))
            .ok_or(strategy::Error::BadOperation)?;
        let records = self
            .backend_op
            .query_by_range(&stock_id, calc_date, end_date)?;
        let views = view::BollingerBandView::transform(&records)?;

        if records.len() < PERIOD {
            return Ok(vec![]);
        }

        for (index, view) in views.iter().enumerate() {
            if view.date < start_date {
                continue;
            }
            return Ok(Vec::from_iter(views[index..views.len()].iter().cloned()));
        }
        Ok(vec![])
    }
}

impl strategy::StrategyAPI for Strategy {
    fn analyze(
        &self,
        stock_id: &str,
        assess_date: chrono::NaiveDate,
    ) -> Result<strategy::Score, strategy::Error> {
        let analyze_date = assess_date
            .checked_sub_signed(chrono::Duration::days(ANALYZE_RANGE as i64 * 2))
            .ok_or(strategy::Error::BadOperation)?;
        let mut score = strategy::Score::default();
        let views = self.get_views(stock_id, analyze_date, assess_date)?;

        if views.len() < ANALYZE_RANGE {
            return Ok(score);
        }

        let last_view = views.last().unwrap();

        if last_view.date != assess_date {
            return Ok(score);
        }

        let mut tmp_sd = last_view.sd;
        let mut rise_ratio = 0.0;
        let mut in_buy_zone_ratio = 0.0;
        let mut total_count = 0;
        let mut in_buy_zone_count = 0;

        for view in views.iter().rev() {
            let price = (view.high + view.low + view.close) / 3.0;

            if price == 0.0 {
                return Ok(score);
            }
            if tmp_sd < view.sd {
                return Ok(score);
            }

            tmp_sd = view.sd;
            total_count = total_count + 1;
            if price >= view.sma + view.sd && price <= view.sma + BAND_SIZE as f64 * view.sd {
                in_buy_zone_count = in_buy_zone_count + 1;
            }

            if total_count == ANALYZE_RANGE {
                in_buy_zone_ratio = (in_buy_zone_count as f64 / total_count as f64) * 100.0;
                rise_ratio = (last_view.sma - view.sma) / view.sma * 100.0;
                break;
            }
        }

        if rise_ratio <= 0.0 {
            return Ok(score);
        }

        score.point = (in_buy_zone_ratio * rise_ratio) as i64;
        score.trading_volume = last_view.volume;
        Ok(score)
    }

    fn settle_check(
        &self,
        stock_id: &str,
        hold_date: chrono::NaiveDate,
        assess_date: chrono::NaiveDate,
    ) -> Result<bool, strategy::Error> {
        let views = self.get_views(stock_id, hold_date, assess_date)?;

        if views.len() == 0 {
            return Ok(false);
        }
        if views.last().unwrap().date != assess_date {
            return Ok(false);
        }

        const CONT_LOW_LIMIT: i32 = 3;
        let mut count = 0;

        for view in views.iter().rev() {
            let price = view.low + (view.high - view.low) * 0.75;

            if price >= view.sma + view.sd {
                break;
            }

            count = count + 1;
            if count == CONT_LOW_LIMIT {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn draw_view(&self, stock_id: &str) -> Result<(), strategy::Error> {
        let records = self.backend_op.query_all(stock_id)?;
        let views = view::BollingerBandView::transform(&records)?;
        let mut date_series = Vec::new();
        let mut open_series = Vec::new();
        let mut high_series = Vec::new();
        let mut low_series = Vec::new();
        let mut close_series = Vec::new();
        let mut sma_series = Vec::new();
        let mut upper_band_series = Vec::new();
        let mut upper_one_sd_band_series = Vec::new();
        let mut lower_band_series = Vec::new();
        let mut lower_one_sd_band_series = Vec::new();
        let mut plot = plotly::Plot::new();

        for view in views {
            date_series.push(view.date.format("%Y-%m-%d").to_string());
            open_series.push(view.open);
            high_series.push(view.high);
            low_series.push(view.low);
            close_series.push(view.close);
            sma_series.push(view.sma);
            upper_band_series.push(view.sma + BAND_SIZE as f64 * view.sd);
            upper_one_sd_band_series.push(view.sma + view.sd);
            lower_band_series.push(view.sma - BAND_SIZE as f64 * view.sd);
            lower_one_sd_band_series.push(view.sma - view.sd);
        }

        let trace_1 = Box::new(
            plotly::Candlestick::new(
                date_series.clone(),
                open_series.clone(),
                high_series.clone(),
                low_series.clone(),
                close_series.clone(),
            )
            .name("Candlestick"),
        );
        let trace_2 = plotly::Scatter::new(date_series.clone(), sma_series.clone())
            .mode(plotly::common::Mode::Lines)
            .name("20 Period SMA");
        let trace_3 = plotly::Scatter::new(date_series.clone(), upper_band_series.clone())
            .mode(plotly::common::Mode::Lines)
            .name(&("Upper Band (".to_owned() + &BAND_SIZE.to_string() + "sd)"));
        let trace_4 = plotly::Scatter::new(date_series.clone(), upper_one_sd_band_series.clone())
            .mode(plotly::common::Mode::Lines)
            .name("Upper Band (1 sd)");
        let trace_5 = plotly::Scatter::new(date_series.clone(), lower_band_series.clone())
            .mode(plotly::common::Mode::Lines)
            .name(&("Lower Band (".to_owned() + &BAND_SIZE.to_string() + "sd)"));
        let trace_6 = plotly::Scatter::new(date_series.clone(), lower_one_sd_band_series.clone())
            .mode(plotly::common::Mode::Lines)
            .name("Upper Band (1 sd)");

        plot.add_trace(trace_1);
        plot.add_trace(trace_2);
        plot.add_trace(trace_3);
        plot.add_trace(trace_4);
        plot.add_trace(trace_5);
        plot.add_trace(trace_6);
        plot.show();

        Ok(())
    }
}
