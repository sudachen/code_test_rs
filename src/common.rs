use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use thiserror::Error;

#[derive(Copy, Clone, Default, PartialEq, Debug, Eq, Hash, Serialize, Deserialize)]
pub struct Client(pub u16);
impl From<u32> for Client {
    fn from(v: u32) -> Self {
        Client(v as u16)
    }
}

#[derive(Copy, Clone, Default, PartialEq, Debug, Eq, Hash, Serialize, Deserialize)]
pub struct TxId(pub u32);
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
    #[error("")]
    Empty,
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

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct Account {
    pub available: Decimal,
    pub total: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TxState {
    #[default]
    Committed, // can be disputed
    Disputed,  // disputed
    Finalized, // can not be disputed
    Cancelled, // the transaction amount is not longer count in client account
}

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub client: Client,
    pub amount: Decimal,
    pub state: TxState,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Policy {
    pub allow_negative_balance_for_dispute: bool,
}

pub type IterResult<T> = Result<T, std::io::Error>;
pub trait Ledger {
    fn policy(&self) -> Policy;
    fn get_account(&self, client: Client) -> Result<Option<Account>, std::io::Error>;
    fn put_account(&mut self, client: Client, account: Account) -> Result<(), std::io::Error>;
    fn accounts<'q>(&'q self) -> Box<dyn Iterator<Item = IterResult<(Client, Account)>> + 'q>;
    fn get_transaction(&self, tx_id: TxId) -> Result<Option<Transaction>, std::io::Error>;
    fn put_transaction(&mut self, tx_id: TxId, tx: Transaction) -> Result<(), std::io::Error>;
    fn transactions<'q>(&'q self)
        -> Box<dyn Iterator<Item = IterResult<(TxId, Transaction)>> + 'q>;

    fn deposit(&mut self, client: Client, tx_id: TxId, amount: Decimal) -> Result<(), TxError> {
        let opt_acc = self.get_account(client)?;
        if self.get_transaction(tx_id)?.is_some() {
            return Err(TxError::Ignored("duplicated transaction".to_string()));
        }
        if let Some(acc) = &opt_acc {
            if acc.locked {
                return Err(TxError::Rejected("account is locked".to_string()));
            }
        }
        self.put_transaction(
            tx_id,
            Transaction {
                client,
                amount,
                state: TxState::Committed,
            },
        )?;
        self.put_account(
            client,
            match opt_acc {
                Some(acc) => Account {
                    available: amount + acc.available,
                    total: amount + acc.total,
                    ..acc
                },
                None => Account {
                    available: amount,
                    total: amount,
                    ..Default::default()
                },
            },
        )?;
        Ok(())
    }
    fn withdrawal(&mut self, client: Client, tx_id: TxId, amount: Decimal) -> Result<(), TxError> {
        let opt_acc = self.get_account(client)?;
        match opt_acc {
            None => Err(TxError::Rejected("account does not exist".to_string())),
            Some(acc) if acc.locked => Err(TxError::Rejected("account is locked".to_string())),
            Some(_) if self.get_transaction(tx_id)?.is_some() => {
                Err(TxError::Ignored("duplicated transaction".to_string()))
            }
            Some(acc) if acc.available < amount => {
                Err(TxError::Rejected("insufficient funds".to_string()))
            }
            Some(acc) => {
                // store transaction for prevent double spending only,
                // it can not be disputed
                self.put_transaction(
                    tx_id,
                    Transaction {
                        client,
                        amount,
                        state: TxState::Finalized,
                    },
                )?;
                self.put_account(
                    client,
                    Account {
                        available: acc.available - amount,
                        total: acc.total - amount,
                        ..acc
                    },
                )?;
                Ok(())
            }
        }
    }
    fn dispute(&mut self, client: Client, tx_id: TxId) -> Result<(), TxError> {
        let (tx, acc) = self.get_and_check_tx_acc(client, tx_id, TxState::Committed)?;
        self.put_account(
            client,
            Account {
                available: acc.available - tx.amount,
                held: acc.held + tx.amount,
                ..acc
            },
        )?;
        // TODO: this means any IO error place storage into "required to repair" state
        // TODO: held/available may be corrected by summing dispute transactions after
        // TODO: repair
        self.put_transaction(
            tx_id,
            Transaction {
                state: TxState::Disputed,
                ..tx
            },
        )?;
        Ok(())
    }
    fn resolve(&mut self, client: Client, tx_id: TxId) -> Result<(), TxError> {
        let (tx, acc) = self.get_and_check_tx_acc(client, tx_id, TxState::Disputed)?;
        self.put_account(
            client,
            Account {
                available: acc.available + tx.amount,
                held: acc.held - tx.amount,
                ..acc
            },
        )?;
        // TODO: this means any IO error place storage into "requires repair" state
        // TODO: held/available may be corrected by summing dispute transactions after
        // TODO: repair
        self.put_transaction(
            tx_id,
            Transaction {
                // TODO: if it can be disputed again it must be TxState::Committed
                state: TxState::Finalized,
                ..tx
            },
        )?;
        Ok(())
    }
    fn chargeback(&mut self, client: Client, tx_id: TxId) -> Result<(), TxError> {
        let (tx, acc) = self.get_and_check_tx_acc(client, tx_id, TxState::Disputed)?;
        self.put_account(
            client,
            Account {
                total: acc.total - tx.amount,
                held: acc.held - tx.amount,
                locked: true,
                ..acc
            },
        )?;
        // TODO: this means any IO error place storage into "requires repair" state
        // TODO: held/available may be corrected by summing dispute transactions after
        // TODO: repair
        self.put_transaction(
            tx_id,
            Transaction {
                state: TxState::Cancelled,
                ..tx
            },
        )?;
        Ok(())
    }
    fn get_and_check_tx_acc(
        &self,
        client: Client,
        tx_id: TxId,
        tx_state: TxState,
    ) -> Result<(Transaction, Account), TxError> {
        let opt_tx = self.get_transaction(tx_id)?;
        let opt_acc = self.get_account(client)?;
        match (opt_tx, opt_acc) {
            (None, _) => Err(TxError::Rejected(
                "deposit transaction does not exist".to_string(),
            )),
            (_, None) => Err(TxError::Rejected(
                "disputed account does not exist".to_string(),
            )),
            (Some(tx), _) if tx.client != client => Err(TxError::Rejected(
                "malicious transaction, wrong client".to_string(),
            )),
            (Some(tx), _) if tx.state != tx_state => match tx_state {
                TxState::Committed if tx.state == TxState::Disputed => {
                    Err(TxError::Ignored("already disputed".to_string()))
                }
                TxState::Disputed => {
                    Err(TxError::Rejected("transaction is not disputed".to_string()))
                }
                _ => Err(TxError::Rejected("can not be disputed".to_string())),
            },
            // TODO: unknown case
            (_, Some(acc)) if acc.locked => Err(TxError::Rejected("account is locked".to_string())),
            // TODO: unknown case
            (Some(tx), Some(acc))
                if !self.policy().allow_negative_balance_for_dispute
                && tx.state == TxState::Committed /* we do dispute */
                && tx.amount > acc.available =>
            {
                Err(TxError::Rejected(
                    "insufficient funds for dispute".to_string(),
                ))
            }
            (Some(tx), Some(acc)) => Ok((tx, acc)),
        }
    }
}
