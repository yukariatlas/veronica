extern crate getopts;

use std::rc::Rc;

use veronica::config::config;
use veronica::storage::backend;
use veronica::strategy::strategy::{self, StrategyAPI};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();

    opts.reqopt("c", "config", "set config path", "");
    opts.reqopt("s", "stock_id", "set stock id", "");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => {
            println!("{}", f);
            return;
        }
    };

    let stock_id = matches.opt_str("s").unwrap();
    let config = config::load_config(&matches.opt_str("c").unwrap()).unwrap();
    let backend_op = Rc::new(backend::SledBackend::new(&config.db_path).unwrap());
    let strategy = Rc::new(strategy::StrategyFactory::get(strategy::Strategies::BollingerBand, backend_op.clone()));

    strategy.draw_view(&stock_id).unwrap();
}
