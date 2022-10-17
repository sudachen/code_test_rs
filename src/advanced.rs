use crate::common::{Ledger as LedgerTrait, *};
use core::default::Default;

#[derive(Clone, Debug)]
pub struct Ledger(sled::Db);

impl Default for Ledger {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Ledger {
    pub fn open(path: String) -> sled::Result<Ledger> {
        sled::Config::default()
            .path(path)
            .open()
            .map(|db| Ledger(db))
    }
    pub fn new_empty(path: Option<String>) -> sled::Result<Ledger> {
        match path {
            Some(path) => match sled::Config::default().path(path).open() {
                Ok(db) => match db.clear() {
                    Ok(_) => Ok(db),
                    Err(e) => Err(e),
                },
                e => e,
            },
            None => sled::Config::default().temporary(true).open(),
        }
        .map(|db| Ledger(db))
    }
    pub fn new() -> sled::Result<Ledger> {
        Self::new_empty(None)
    }
}

use serde::Deserialize;
use std::io::{Error as IoError, ErrorKind::Other as AnotherError};

impl<'q> LedgerTrait<'q> for Ledger {
    fn get_account(&self, client: Client) -> Result<Option<Account>, IoError> {
        get(&self.0.get(format!("1'{:?}", client)))
    }
    fn put_account(&mut self, client: Client, account: Account) -> Result<(), IoError> {
        // we can simple ignore errors on serialization here
        match self
            .0
            .insert(format!("1'{:?}", client), bson::to_vec(&account).unwrap())
        {
            Err(e) => Err(std::io::Error::new(AnotherError, e)),
            _ => Ok(()),
        }
    }
    fn accounts(&'q self) -> Box<dyn Iterator<Item = IterResult<(Client, Account)>> + 'q> {
        Box::new(self.0.range("1'0".."2'0").map(|v| decode(&v)))
    }
    fn get_transaction(&self, tx_id: TxId) -> Result<Option<Transaction>, std::io::Error> {
        get(&self.0.get(format!("2'{:?}", tx_id)))
    }
    fn put_transaction(&mut self, tx_id: TxId, tx: Transaction) -> Result<(), std::io::Error> {
        // we can simple ignore errors on serialization here
        match self
            .0
            .insert(format!("2'{:?}", tx_id), bson::to_vec(&tx).unwrap())
        {
            Err(e) => Err(std::io::Error::new(AnotherError, e)),
            _ => Ok(()),
        }
    }
    fn transactions(&'q self) -> Box<dyn Iterator<Item = IterResult<(TxId, Transaction)>> + 'q> {
        Box::new(self.0.range("2'0"..).map(|v| decode(&v)))
    }
}

fn decode<'a, A: Deserialize<'a>, B: Deserialize<'a>>(
    v: &'a sled::Result<(sled::IVec, sled::IVec)>,
) -> IterResult<(A, B)> {
    match v {
        Ok((a, b)) => match (bson::from_slice(a), bson::from_slice(b)) {
            (Ok(a), Ok(b)) => Ok((a, b)),
            (Err(e), _) => Err(std::io::Error::new(AnotherError, e)),
            (Ok(_), Err(e)) => Err(std::io::Error::new(AnotherError, e)),
        },
        Err(e) => Err(std::io::Error::new(AnotherError, e.clone())),
    }
}

fn get<'a, T: Deserialize<'a>>(
    v: &'a sled::Result<Option<sled::IVec>>,
) -> Result<Option<T>, IoError> {
    match v {
        Err(e) => Err(std::io::Error::new(AnotherError, e.clone())),
        Ok(None) => Ok(None),
        Ok(Some(a)) => match bson::from_slice(a) {
            Err(e) => Err(std::io::Error::new(AnotherError, e)),
            Ok(a) => Ok(Some(a)),
        },
    }
}
