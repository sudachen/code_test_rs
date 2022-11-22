use crate::common::*;
use crate::libcsv::{validate_accounts_internal, AccountState, ExecError, TxRequest};
use crossbeam::sync::WaitGroup;
use crossbeam_channel::{bounded, unbounded, Sender, TryRecvError};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::io::{Error as IoError, ErrorKind::Other as AnotherError};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

const MSG_QUEUE_LENGTH: usize = 8;

pub fn index_by_client(c: Client, concurrency: usize) -> usize {
    let index = c.0 as u32;
    let r = (((index + 1013904223) as u64) * 1664525) as u32;
    (r as usize * concurrency) >> 32
}

pub fn sharded_validate_accounts(
    rd: impl std::io::Read,
    ledgers: &Vec<Arc<Mutex<dyn Ledger + Send>>>,
    index: impl Fn(Client, usize) -> usize,
) -> Result<(), ExecError> {
    let concurrency = ledgers.len();
    validate_accounts_internal(rd, |c| {
        ledgers[index(c, concurrency)]
            .lock()
            .unwrap()
            .get_account(c)
    })
}

pub fn sharded_dump_accounts(
    wr: impl std::io::Write,
    ledgers: &Vec<Arc<Mutex<dyn Ledger + Send>>>,
    index: impl Fn(Client, usize) -> usize,
) -> Result<(), ExecError> {
    let mut wrr = csv::WriterBuilder::new().delimiter(b',').from_writer(wr);
    let index = |c| index(c, ledgers.len());
    for (i, l) in ledgers.iter().enumerate() {
        for pair in l.lock().unwrap().accounts() {
            match pair {
                Ok((client, _)) if index(client) != i => Ok(()),
                Ok((client, state)) => wrr.serialize(AccountState {
                    client,
                    available: state.available,
                    total: state.total,
                    held: state.held,
                    locked: state.locked,
                }),
                Err(e) => Err(e.into()),
            }?;
        }
    }
    Ok(())
}

pub fn sharded_execute_csv_file(
    path: impl AsRef<Path>,
    ledgers: &Vec<Arc<Mutex<dyn Ledger + Send>>>,
    index: impl Fn(Client, usize) -> usize,
) -> Result<(), ExecError> {
    let mut f = std::fs::File::open(path)?;
    sharded_execute_csv(&mut f, ledgers, index)
}

pub fn sharded_execute_csv(
    rd: impl std::io::Read,
    ledgers: &Vec<Arc<Mutex<dyn Ledger + Send>>>,
    index: impl Fn(Client, usize) -> usize,
) -> Result<(), ExecError> {
    let mut ch: Vec<Sender<TxRequest>> = Vec::new();
    let wg = WaitGroup::new();
    let (res_s, res_r) = unbounded::<ExecError>();
    for ledger in ledgers {
        let res_s = res_s.clone();
        let (msg_s, msg_r) = bounded(MSG_QUEUE_LENGTH);
        ch.push(msg_s);
        let wg = wg.clone();
        let ledger = ledger.clone();
        thread::spawn(move || {
            let mut l = ledger.lock().unwrap();
            loop {
                use TxType::*;
                let res = match msg_r.recv() {
                    Ok(tx) => match tx.tx_type {
                        Deposit => l.deposit(tx.client, tx.tx_id, tx.amount.unwrap()),
                        Withdrawal => l.withdrawal(tx.client, tx.tx_id, tx.amount.unwrap()),
                        Dispute => l.dispute(tx.client, tx.tx_id),
                        Resolve => l.resolve(tx.client, tx.tx_id),
                        Chargeback => l.chargeback(tx.client, tx.tx_id),
                    },
                    Err(_) => Err(TxError::Empty),
                };
                match res {
                    Ok(_) | Err(TxError::Rejected(_)) | Err(TxError::Ignored(_)) => (), // ignore
                    Err(TxError::Empty) => break,
                    Err(e) => {
                        let _ = res_s.try_send(e.into());
                        break;
                    }
                }
            }
            drop(wg);
        });
    }
    let concurrency = ledgers.len();
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .comment(Some(b'#'))
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(rd);
    for result in rdr.deserialize() {
        let r: TxRequest = result?;
        use TxType::*;
        let wkr = index(r.client, concurrency);
        match (r.tx_type, r.amount) {
            (Deposit | Withdrawal, None) => Err(ExecError::StringError("tx has no amount".into())),
            _ => Ok(r),
        }
        .and_then(|x| match res_r.try_recv() {
            Err(TryRecvError::Empty) => ch[wkr]
                .send(x)
                .map_err(|e| ExecError::StringError(e.to_string())),
            Ok(err) => Err(err),
            Err(err) => Err(ExecError::StringError(err.to_string())),
        })?;
    }
    drop(ch); // close all channels
    wg.wait();
    match res_r.try_recv() {
        Err(TryRecvError::Empty) => Ok(()),
        Ok(err) => Err(err),
        Err(err) => Err(ExecError::StringError(err.to_string())),
    }
}

#[derive(Clone, Debug)]
pub struct SledLedger(sled::Db, Policy);

impl Default for SledLedger {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl SledLedger {
    #[allow(dead_code)]
    pub fn open(path: String, policy: Policy) -> sled::Result<SledLedger> {
        sled::Config::default()
            .path(path)
            .open()
            .map(|db| SledLedger(db, policy))
    }
    pub fn new_empty(path: Option<String>, policy: Policy) -> sled::Result<SledLedger> {
        match path {
            Some(path) => sled::Config::default().path(path).open().and_then(|db| {
                db.clear()?;
                Ok(db)
            }),
            None => sled::Config::default().temporary(true).open(),
        }
        .map(|db| SledLedger(db, policy))
    }
    #[allow(dead_code)]
    pub fn new() -> sled::Result<SledLedger> {
        Self::new_empty(None, Default::default())
    }
    #[allow(dead_code)]
    pub fn sharding(&self, n: usize) -> Vec<Arc<Mutex<dyn Ledger + Send>>> {
        (0..n)
            .map(|_| Arc::new(Mutex::new(self.clone())) as _)
            .collect()
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Rec<K, V> {
    k: K,
    v: V,
}
type AccRec = Rec<Client, Account>;
type TxRec = Rec<TxId, Transaction>;

impl Ledger for SledLedger {
    fn policy(&self) -> Policy {
        self.1
    }
    fn get_account(&self, client: Client) -> Result<Option<Account>, IoError> {
        get::<AccRec>(&self.0.get(format!("1'{:?}", client))).map(|x| x.map(|r| r.v))
    }
    fn put_account(&mut self, client: Client, account: Account) -> Result<(), IoError> {
        // we can simple ignore errors on serialization here
        self.0
            .insert(
                format!("1'{:?}", client),
                bson::to_vec(&AccRec {
                    k: client,
                    v: account,
                })
                .unwrap(),
            )
            .map_err(|e| std::io::Error::new(AnotherError, e))?;
        Ok(())
    }
    fn accounts<'q>(&'q self) -> Box<dyn Iterator<Item = IterResult<(Client, Account)>> + 'q> {
        Box::new(self.0.range("1'0".."2'0").map(|v| decode(&v)))
    }
    fn get_transaction(&self, tx_id: TxId) -> Result<Option<Transaction>, std::io::Error> {
        get::<TxRec>(&self.0.get(format!("2'{:?}", tx_id))).map(|x| x.map(|tx| tx.v))
    }
    fn put_transaction(&mut self, tx_id: TxId, tx: Transaction) -> Result<(), std::io::Error> {
        // we can simple ignore errors on serialization here
        self.0
            .insert(
                format!("2'{:?}", tx_id),
                bson::to_vec(&TxRec { k: tx_id, v: tx }).unwrap(),
            )
            .map_err(|e| std::io::Error::new(AnotherError, e))?;
        Ok(())
    }
    fn transactions<'q>(
        &'q self,
    ) -> Box<dyn Iterator<Item = IterResult<(TxId, Transaction)>> + 'q> {
        Box::new(self.0.range("2'0"..).map(|v| decode(&v)))
    }
}

fn decode<'a, A: Deserialize<'a>, B: Deserialize<'a>>(
    v: &'a sled::Result<(sled::IVec, sled::IVec)>,
) -> IterResult<(A, B)> {
    match v {
        Ok((_, b)) => match bson::from_slice::<'a, Rec<A, B>>(b) {
            Ok(r) => Ok((r.k, r.v)),
            Err(e) => Err(std::io::Error::new(AnotherError, e)),
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

#[test]
fn test_concurrent_csv_processing_1() -> Result<(), ExecError> {
    let ledger = SledLedger::new().unwrap();
    let sharding = ledger.sharding(3);
    sharded_execute_csv(
        std::io::Cursor::new(crate::basic::TRANSACTIONS.as_bytes()),
        &sharding,
        index_by_client,
    )?;
    sharded_validate_accounts(
        std::io::Cursor::new(crate::basic::ACCOUNTS.as_bytes()),
        &sharding,
        index_by_client,
    )?;
    sharded_dump_accounts(std::io::stdout(), &sharding, index_by_client)?;
    Ok(())
}

#[test]
fn test_concurrent_csv_processing_2() -> Result<(), ExecError> {
    let sharding = (0..3)
        .map(|_| {
            Arc::new(Mutex::new(crate::basic::HashLedger::default()))
                as _
        })
        .collect();
    sharded_execute_csv(
        std::io::Cursor::new(crate::basic::TRANSACTIONS.as_bytes()),
        &sharding,
        index_by_client,
    )?;
    sharded_validate_accounts(
        std::io::Cursor::new(crate::basic::ACCOUNTS.as_bytes()),
        &sharding,
        index_by_client,
    )?;
    sharded_dump_accounts(std::io::stdout(), &sharding, index_by_client)?;
    Ok(())
}
