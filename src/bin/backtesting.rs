extern crate getopts;

use std::rc::Rc;

use veronica::config::config;
use veronica::core::{backtesting, utils};
use veronica::storage::backend;
use veronica::crawler::finmind;

fn update_raw_data(config: &config::Config, start_date: chrono::NaiveDate, end_date: chrono::NaiveDate)
{
    let crawler = Rc::new(finmind::Finmind::new(&config.finmind_token));
    let backend_op = Rc::new(backend::SledBackend::new(&config.db_path).unwrap());
    let utils = utils::Utils::new(crawler.clone(), backend_op.clone());

    utils.update_raw_data(start_date, end_date).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();

    opts.reqopt("c", "config", "set config path", "");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => {
            println!("{}", f);
            return;
        }
    };
    let config = config::load_config(&matches.opt_str("c").unwrap()).unwrap();

    update_raw_data(
        &config,
        chrono::NaiveDate::from_ymd(2021, 1, 1),
        chrono::NaiveDate::from_ymd(2021, 12, 31) 
    );

    let mut backtesting = backtesting::Backtesting {
        config: config,
        start_date: chrono::NaiveDate::from_ymd(2021, 6, 1),
        end_date: chrono::NaiveDate::from_ymd(2021, 12, 31),
        ..Default::default()
    };

    backtesting.run();
}