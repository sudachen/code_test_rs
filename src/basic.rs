use crate::common::{Accountant as AccountantTrait, Ledger as LedgerTrait, *};
use core::default::Default;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Accountant<L: Clone + for<'q> LedgerTrait<'q> = Ledger> {
    policy: Policy,
    ledger: L,
}

impl Default for Accountant {
    fn default() -> Accountant {
        return Self::new(Ledger::default());
    }
}

impl<L: for<'q> LedgerTrait<'q> + Clone> AccountantTrait for Accountant<L> {
    fn deposit(&mut self, client: Client, tx_id: TxId, amount: Decimal) -> Result<(), TxError> {
        let opt_acc = self.ledger.get_account(client)?;
        if self.ledger.get_transaction(tx_id)?.is_some() {
            return Err(TxError::Ignored("duplicated transaction".to_string()));
        }
        if let Some(acc) = &opt_acc {
            if acc.locked {
                return Err(TxError::Rejected("account is locked".to_string()));
            }
        }
        self.ledger.put_transaction(
            tx_id,
            Transaction {
                client,
                amount,
                state: TxState::Committed,
            },
        )?;
        self.ledger.put_account(
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
        let opt_acc = self.ledger.get_account(client)?;
        match opt_acc {
            None => Err(TxError::Rejected("account does not exist".to_string())),
            Some(acc) if acc.locked => Err(TxError::Rejected("account is locked".to_string())),
            Some(_) if self.ledger.get_transaction(tx_id)?.is_some() => {
                Err(TxError::Ignored("duplicated transaction".to_string()))
            }
            Some(acc) if acc.available < amount => {
                Err(TxError::Rejected("insufficient funds".to_string()))
            }
            Some(acc) => {
                // store transaction for prevent double spending only,
                // it can not be disputed
                self.ledger.put_transaction(
                    tx_id,
                    Transaction {
                        client,
                        amount,
                        state: TxState::Finalized,
                    },
                )?;
                self.ledger.put_account(
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
        self.ledger.put_account(
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
        self.ledger.put_transaction(
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
        self.ledger.put_account(
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
        self.ledger.put_transaction(
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
        self.ledger.put_account(
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
        self.ledger.put_transaction(
            tx_id,
            Transaction {
                state: TxState::Cancelled,
                ..tx
            },
        )?;
        Ok(())
    }
    fn ledger(&self) -> &dyn LedgerTrait {
        &self.ledger
    }
}

impl<L: for<'q> LedgerTrait<'q> + Clone> Accountant<L> {
    fn get_and_check_tx_acc(
        &self,
        client: Client,
        tx_id: TxId,
        tx_state: TxState,
    ) -> Result<(Transaction, Account), TxError> {
        let opt_tx = self.ledger.get_transaction(tx_id)?;
        let opt_acc = self.ledger.get_account(client)?;
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
                if self.policy.allow_negative_balance_for_dispute != true
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
    pub fn new(ledger: L) -> Self {
        Self {
            ledger,
            policy: Default::default(),
        }
    }
    pub fn with_policy(ledger: L, policy: Policy) -> Self {
        Self { ledger, policy }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Ledger {
    transactions: HashMap<TxId, Transaction>,
    accounts: HashMap<Client, Account>,
}

impl<'q> LedgerTrait<'q> for Ledger {
    fn get_account(&self, client: Client) -> Result<Option<Account>, std::io::Error> {
        Ok(self.accounts.get(&client).copied())
    }
    fn put_account(&mut self, client: Client, account: Account) -> Result<(), std::io::Error> {
        self.accounts.insert(client, account);
        Ok(())
    }
    fn accounts(&'q self) -> Box<dyn Iterator<Item = IterResult<(&'q Client, &'q Account)>> + 'q> {
        Box::new(self.accounts.iter().map(|v| Ok(v)))
    }
    fn get_transaction(&self, tx_id: TxId) -> Result<Option<Transaction>, std::io::Error> {
        Ok(self.transactions.get(&tx_id).copied())
    }
    fn put_transaction(&mut self, tx_id: TxId, tx: Transaction) -> Result<(), std::io::Error> {
        self.transactions.insert(tx_id, tx);
        Ok(())
    }
    fn transactions(
        &'q self,
    ) -> Box<dyn Iterator<Item = IterResult<(&'q TxId, &'q Transaction)>> + 'q> {
        Box::new(self.transactions.iter().map(|v| Ok(v)))
    }
}
