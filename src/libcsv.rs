use rust_decimal::Decimal;
use crate::common::{Client, TxId, TxType, Bank, TxError, Account};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Deserialize)]
struct TxRequest {
    #[serde(rename="type")]
    tx_type: TxType,
    client: Client,
    #[serde(rename="tx")]
    tx_id: TxId,
    amount: Decimal
}

#[derive(Deserialize,Serialize)]
struct AccountState {
    client: Client,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool
}

#[derive(Error, Debug)]
pub enum ExecError {
    #[error("{0}")]
    StringError(String),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    CSVError(#[from] csv::Error),
    #[error(transparent)]
    TxError(#[from] TxError),
}

pub fn execute_csv_file(path: impl AsRef<Path>, bank: &mut dyn Bank) -> Result<(),ExecError> {
    let mut f = std::fs::File::open(path)?;
    execute_csv(&mut f, bank)
}

pub fn execute_csv(rd: impl std::io::Read, bank: &mut dyn Bank) -> Result<(),ExecError>{
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .trim(csv::Trim::All)
        .from_reader(rd);
    for result in rdr.deserialize() {
        let r: TxRequest = result?;
        use TxType::*;
        let err = match r.tx_type {
            Deposit => bank.deposit(r.client,r.tx_id,r.amount),
            Withdrawal => bank.withdrawal(r.client,r.tx_id,r.amount),
            Dispute => bank.dispute(r.client,r.tx_id),
            Resolve => bank.resolve(r.client,r.tx_id),
            Chargeback => bank.chargeback(r.client,r.tx_id)
        };
        match err {
            Err(TxError::Rejected(_e)) => { Ok(())}
            Err(TxError::Ignored(_e)) => { Ok(())}
            e => e
        }?;
    }
    Ok(())
}

pub fn validate_accounts(rd: impl std::io::Read, bank: &mut dyn Bank) -> Result<(),ExecError>{
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .trim(csv::Trim::All)
        .from_reader(rd);
    for result in rdr.deserialize() {
        let r: AccountState = result?;
        let client = r.client;
        match bank.ledger().get_account(client)? {
            None => Err(ExecError::StringError("".into())),
            Some(Account{available,total,held,locked}) =>
                if available == r.available && total == r.total &&
                    held == r.held && locked == r.locked {
                    Ok(())
                } else {
                    Err(ExecError::StringError("account state does not match to csv record".into()))
                }
        }?
    }
    Ok(())
}

pub fn dump_accounts(wr: impl std::io::Write, bank: &mut dyn Bank) -> Result<(),ExecError>{
    let mut wrr = csv::WriterBuilder::new()
        .delimiter(b',')
        .from_writer(wr);
    for (&client,state) in bank.ledger().accounts() {
        wrr.serialize(AccountState{
            client,
            available: state.available,
            total: state.total,
            held: state.held,
            locked: state.locked})?
    }
    Ok(())
}
