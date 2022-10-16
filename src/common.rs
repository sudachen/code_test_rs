use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use thiserror::Error;

#[derive(Copy, Clone, Default, PartialEq, Debug, Eq, Hash, Serialize, Deserialize)]
pub struct Client(u16);
impl From<u32> for Client {
    fn from(v: u32) -> Self {
        Client(v as u16)
    }
}

#[derive(Copy, Clone, Default, PartialEq, Debug, Eq, Hash, Serialize, Deserialize)]
pub struct TxId(u32);
impl From<u32> for TxId {
    fn from(v: u32) -> Self {
        TxId(v as u32)
    }
}

#[derive(Error, Debug)]
pub enum TxError {
    #[error("{0}")]
    StringError(String),
    #[error("Transaction rejected: {0}")]
    Rejected(String),
    #[error("Transaction ignored: {0}")]
    Ignored(String),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum TxType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Account {
    pub available: Decimal,
    pub total: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

#[derive(Copy, Clone, Default, PartialEq, Debug)]
pub enum TxState {
    #[default]
    Committed, // can be disputed
    Disputed,  // disputed
    Finalized, // can not be disputed
    Cancelled, // the transaction amount is not longer count in client account
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Transaction {
    pub client: Client,
    pub amount: Decimal,
    pub state: TxState,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Policy {
    pub allow_negative_balance_for_dispute: bool
}

pub trait Bank {
    fn deposit(&mut self, client: Client, tx_id: TxId, amount: Decimal) -> Result<(), TxError>;
    fn withdrawal(&mut self, client: Client, tx_id: TxId, amount: Decimal) -> Result<(), TxError>;
    fn dispute(&mut self, client: Client, tx_id: TxId) -> Result<(), TxError>;
    fn resolve(&mut self, client: Client, tx_id: TxId) -> Result<(), TxError>;
    fn chargeback(&mut self, client: Client, tx_id: TxId) -> Result<(), TxError>;
    fn ledger(&self) -> &dyn Ledger;
}

pub trait Ledger<'q> {
    fn get_account(&self, client: Client) -> Result<Option<Account>, std::io::Error>;
    fn put_account(&mut self, client: Client, account: Account) -> Result<(), std::io::Error>;
    fn accounts(&'q self) -> Box<dyn Iterator<Item = (&'q Client, &'q Account)> + 'q>;
    fn get_transaction(&self, tx_id: TxId) -> Result<Option<Transaction>, std::io::Error>;
    fn put_transaction(&mut self, tx_id: TxId, tx: Transaction) -> Result<(), std::io::Error>;
    fn transactions(&'q self) -> Box<dyn Iterator<Item = (&'q TxId, &'q Transaction)> + 'q>;
}

impl Debug for dyn Bank {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Bank")
    }
}
