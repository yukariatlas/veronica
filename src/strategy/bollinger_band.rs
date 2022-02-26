use std::rc::Rc;

use crate::dataview::view::{self, Transform};
use crate::export::export;
use crate::storage::backend;
use crate::strategy::strategy;

use super::schema::RawData;

pub const PERIOD: usize = 20;
pub const BAND_SIZE: usize = 2;

pub struct Strategy {
    pub backend_op: Rc<dyn backend::BackendOp>,
}

impl Strategy {
    fn get_views(&self, stock_id: &str, start_date: chrono::NaiveDate, end_date: chrono::NaiveDate) -> Result<Vec<view::BollingerBandView>, strategy::Error> {
        let calc_date = start_date.checked_sub_signed(chrono::Duration::days(40)).ok_or(strategy::Error::BadOperation)?;
        let records = self.backend_op.query_by_range(&stock_id, calc_date, end_date)?;
        let views = view::BollingerBandView::transform(&records)?;

        if records.len() < PERIOD {
            return Err(strategy::Error::BadOperation);
        }

        for (index, view) in views.iter().enumerate() {
            if view.date < start_date {
                continue;
            }
            return Ok(Vec::from_iter(views[index..views.len()].iter().cloned()));
        }
        Err(strategy::Error::RecordNotFound)
    }
}

impl strategy::StrategyAPI for Strategy {
    fn analyze(&self, stock_id: &str, assess_date: chrono::NaiveDate) -> Result<strategy::Score, strategy::Error> {
        const ANALYZE_RANGE: usize = 10;
        let analyze_date = assess_date.checked_sub_signed(chrono::Duration::days(20)).ok_or(strategy::Error::BadOperation)?;
        let views = self.get_views(stock_id, analyze_date, assess_date)?;
        let mut score = strategy::Score::default();

        if views.len() < ANALYZE_RANGE {
            return Err(strategy::Error::BadOperation);
        }

        if views[0].sma >= views.last().unwrap().sma {
            return Ok(score);
        }

        let mut total_count = 0;
        let mut in_buy_zone_count = 0;

        for view in views.iter().rev() {
            let price = (view.high + view.low + view.close) / 3.0;

            total_count = total_count + 1;
            if price >= view.sma + view.sd && price <= view.sma + BAND_SIZE as f64 * view.sd {
                in_buy_zone_count = in_buy_zone_count + 1;
            }

            if total_count == ANALYZE_RANGE {
                break;
            }
        }

        let in_buy_zone_fraction = in_buy_zone_count / total_count;
        let rise_ratio = (views.last().unwrap().sma - views[0].sma) / views[0].sma;

        score.point = (in_buy_zone_fraction as f64 * rise_ratio * 100.0) as u64;
        score.trading_volume = views.last().ok_or(strategy::Error::BadOperation)?.volume;        
        Ok(score)
    }

    fn settle_check(&self, stock_id: &str, hold_date: chrono::NaiveDate, assess_date: chrono::NaiveDate) -> Result<bool, strategy::Error> {
        let views = self.get_views(stock_id, hold_date, assess_date)?;
        let assess_view = views.last().unwrap();

        if views[0].sma > assess_view.sma {
            return Ok(true);
        }

        let price = assess_view.low + (assess_view.high - assess_view.low) * 0.75;

        if assess_view.open > assess_view.close && price < assess_view.sma + assess_view.sd {
            return Ok(true);
        }
        Ok(false)
    }

    fn export_view(&self, stock_id: &str, records: &Vec<RawData>) -> Result<(), strategy::Error> {
        let views = view::BollingerBandView::transform(&records)?;
    
        export::to_yaml(&stock_id, &views);
        Ok(())
    }
}