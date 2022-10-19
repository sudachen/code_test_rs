use crate::common::*;
use core::default::Default;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct HashLedger {
    transactions: HashMap<TxId, Transaction>,
    accounts: HashMap<Client, Account>,
    policy: Policy,
}

impl HashLedger {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Default::default()
    }
    #[allow(dead_code)]
    pub fn with_policy(policy: Policy) -> Self {
        Self {
            policy,
            ..Default::default()
        }
    }
}

impl Ledger for HashLedger {
    fn get_account(&self, client: Client) -> Result<Option<Account>, std::io::Error> {
        Ok(self.accounts.get(&client).copied())
    }
    fn put_account(&mut self, client: Client, account: Account) -> Result<(), std::io::Error> {
        self.accounts.insert(client, account);
        Ok(())
    }
    fn accounts<'q>(&'q self) -> Box<dyn Iterator<Item = IterResult<(Client, Account)>> + 'q> {
        Box::new(self.accounts.iter().map(|v| Ok((*v.0, *v.1))))
    }
    fn get_transaction(&self, tx_id: TxId) -> Result<Option<Transaction>, std::io::Error> {
        Ok(self.transactions.get(&tx_id).copied())
    }
    fn put_transaction(&mut self, tx_id: TxId, tx: Transaction) -> Result<(), std::io::Error> {
        self.transactions.insert(tx_id, tx);
        Ok(())
    }
    fn transactions<'q>(
        &'q self,
    ) -> Box<dyn Iterator<Item = IterResult<(TxId, Transaction)>> + 'q> {
        Box::new(self.transactions.iter().map(|v| Ok((*v.0, *v.1))))
    }
    fn policy(&self) -> Policy {
        self.policy
    }
}

#[cfg(test)]
use crate::libcsv::{execute_csv, validate_accounts, ExecError};

#[cfg(test)]
pub const TRANSACTIONS: &str = r#"# CSV sample
type,       client, tx, amount
deposit,    1,      1,  1.0
# 1 -> 1.0/0/1.0/false
deposit,    2,      2,  2.0
# 2 -> 2.0/0/2.0/false
deposit,    3,      3,  3.0
# 3 -> 3.0/0/3.0/false
withdrawal, 1,      4,  1.1
# rejected
withdrawal, 2,      5,  1.1111
# 2 -> 0.8889/0/0.8889/false
dispute,    1,      4,
# 1 -> 0/1.0/1.0/false
resolve,    1,      3
# rejected
resolve,    1,      4
# 1 -> 1.0/0/1.0/false
dispute,    1,      4
# rejected
dispute,    2,      2
# rejected
deposit,    2,      5, 4.1111
# rejected
deposit,    2,      6, 4.1111
# 2 -> 5.0/0/5.0/false
dispute,    2,      2
# 2 -> 3.0/2.0/5.0/false
chargeback, 2,      2
# 2 -> 3.0/0/3.0/true
"#;

#[cfg(test)]
pub const ACCOUNTS: &str = r#"
client,     available,  held, total,  locked
1,          1.0,        0,    1.0,    false
2,          3.0,        0,    3.0,    true
3,          3.0,        0,    3.0,    false
"#;

#[test]
fn test_csv_processing() -> Result<(), ExecError> {
    let mut ledger = HashLedger::new();
    execute_csv(std::io::Cursor::new(TRANSACTIONS.as_bytes()), &mut ledger)?;
    validate_accounts(std::io::Cursor::new(ACCOUNTS.as_bytes()), &ledger)
}
