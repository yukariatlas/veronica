use std::fmt::Debug;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use crate::crawler::crawler;
use crate::storage::backend;

#[derive(Debug)]
pub enum Error {
    Backend(backend::Error),
    Crawler(crawler::Error),
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

pub struct Utils {
    pub crawler: Rc<dyn crawler::Crawler>,
    pub backend_op: Rc<dyn backend::BackendOp>,
}

impl Utils {
    pub fn new(crawler: Rc<dyn crawler::Crawler>, backend_op: Rc<dyn backend::BackendOp>) -> Self {
        Utils {
            crawler: crawler,
            backend_op: backend_op
        }
    }
    pub fn update_raw_data(&self, start_date: chrono::NaiveDate, end_date: chrono::NaiveDate) -> Result<(), Error> {
        let mut data = Vec::new();
        let stock_list = self.crawler.get_stock_list()?;

        for stock_id in stock_list {
            let args = crawler::Args {
                stock_id: stock_id.clone(),
                start_date: start_date,
                end_date: end_date,
            };

            print!("Get info of stock [{}]\n", stock_id);
            loop {
                break match self.crawler.get_stock_data(&args) {
                    Ok(records) => {
                        for record in records {
                            data.push((stock_id.clone(), record));
                        }
                    },
                    Err(err) => match err {
                        crawler::Error::RateLimitReached => {
                            print!("The number of request reaches limitation, sleep one hour and continue...\n");
                            thread::sleep(Duration::from_secs(60 * 60));
                            continue;
                        },
                        _ => return Err(Error::Crawler(err)),
                    }
                }
            }
            self.backend_op.batch_insert(&data)?;
        }
        Ok(())
    }
}