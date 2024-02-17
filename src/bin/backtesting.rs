extern crate getopts;

use std::rc::Rc;

use veronica::config::config;
use veronica::core::backtesting;
use veronica::crawler::finmind;
use veronica::storage::backend;
use veronica::strategy::strategy;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();

    opts.reqopt("c", "config", "set config path", "");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f);
            return;
        }
    };

    let config = config::load_config(&matches.opt_str("c").unwrap()).unwrap();
    let crawler = Rc::new(finmind::Finmind::new(&config.finmind_token));
    let backend_op = Rc::new(backend::SledBackend::new(&config.db_path).unwrap());
    let mut backtesting = backtesting::Backtesting::new(
        config,
        crawler,
        backend_op,
        strategy::Strategies::BollingerBand,
    );

    backtesting.run(chrono::NaiveDate::from_ymd(2021, 6, 1), chrono::NaiveDate::from_ymd(2021, 12, 31));
}
