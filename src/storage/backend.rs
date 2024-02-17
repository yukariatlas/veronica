use crate::strategy::schema;

#[derive(Debug)]
pub enum Error {
    Sled(sled::Error),
    Utf8(std::str::Utf8Error),
    Bincode(bincode::Error),
}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Error {
        Error::Sled(err)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Error {
        Error::Utf8(err)
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Error {
        Error::Bincode(err)
    }
}

#[mockall::automock]
pub trait BackendOp {
    fn batch_insert(&self, records: &Vec<(String, schema::RawData)>) -> Result<(), Error>;
    fn query(
        &self,
        stock_id: &str,
        date: chrono::NaiveDate,
    ) -> Result<Option<schema::RawData>, Error>;
    fn query_by_range(
        &self,
        stock_id: &str,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<schema::RawData>, Error>;
    fn query_all(&self, stock_id: &str) -> Result<Vec<schema::RawData>, Error>;
    fn batch_delete(&self, records: &Vec<(String, chrono::NaiveDate)>) -> Result<(), Error>;
}

pub struct SledBackend {
    db_op: sled::Db,
}

impl SledBackend {
    pub fn new(db_path: &str) -> Result<Self, Error> {
        Ok(SledBackend {
            db_op: sled::open(db_path).unwrap(),
        })
    }
}

impl BackendOp for SledBackend {
    fn batch_insert(&self, records: &Vec<(String, schema::RawData)>) -> Result<(), Error> {
        let mut batch = sled::Batch::default();

        for (stock_id, raw_data) in records {
            let key = stock_id.clone() + "_" + &raw_data.date.to_string();
            let encoded = bincode::serialize(raw_data)?;
            batch.insert(&key[..], encoded);
        }

        self.db_op.apply_batch(batch)?;
        Ok(())
    }
    fn query(
        &self,
        stock_id: &str,
        date: chrono::NaiveDate,
    ) -> Result<Option<schema::RawData>, Error> {
        let key = stock_id.to_owned() + "_" + &date.to_string();

        match self.db_op.get(key)? {
            Some(val) => Ok(Some(bincode::deserialize(&val)?)),
            None => Ok(None),
        }
    }
    fn query_by_range(
        &self,
        stock_id: &str,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<schema::RawData>, Error> {
        let start = stock_id.to_owned() + "_" + &start_date.to_string();
        let end = stock_id.to_owned() + "_" + &end_date.succ_opt().unwrap().to_string();
        let mut iter = self.db_op.range(start..end);
        let mut records = Vec::new();

        while let Some(item) = iter.next() {
            let (_, val) = item?;

            records.push(bincode::deserialize(&val)?);
        }

        Ok(records)
    }
    fn query_all(&self, stock_id: &str) -> Result<Vec<schema::RawData>, Error> {
        let mut iter = self.db_op.scan_prefix(stock_id);
        let mut records = Vec::new();

        while let Some(item) = iter.next() {
            let (_, val) = item?;

            records.push(bincode::deserialize(&val)?);
        }

        Ok(records)
    }
    fn batch_delete(&self, records: &Vec<(String, chrono::NaiveDate)>) -> Result<(), Error> {
        let mut batch = sled::Batch::default();

        for (stock_id, date) in records {
            let key = stock_id.to_owned() + "_" + &date.to_string();
            batch.remove(&key[..]);
        }

        self.db_op.apply_batch(batch)?;
        Ok(())
    }
}
