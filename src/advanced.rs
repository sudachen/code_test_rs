use crate::basic::Accountant as BasicAccountant;
use crate::common::{Accountant as AccountantTrait, Ledger as LedgerTrait, *};
use crate::libcsv::{ExecError, TxRequest};
use crossbeam::sync::WaitGroup;
use crossbeam_channel::{bounded, unbounded, Sender, TryRecvError};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::io::{Error as IoError, ErrorKind::Other as AnotherError};
use std::path::Path;
use std::thread;

const MSG_QUEUE_LENGTH: usize = 8;

#[test]
fn test_concurrent_csv_processing() -> Result<(), ExecError> {
    use crate::libcsv::validate_accounts;
    let sample = r#"# CSV sample
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
    let ledger = Ledger::new().unwrap();
    concurrent_execute_csv(
        Some(3),
        std::io::Cursor::new(sample.as_bytes()),
        &ledger,
        Default::default(),
    )?;
    let accounts = r#"
client,     available,  held, total,  locked
1,          1.0,        0,    1.0,    false
2,          3.0,        0,    3.0,    true
3,          3.0,        0,    3.0,    false
"#;
    validate_accounts(std::io::Cursor::new(accounts.as_bytes()), &ledger)
}

pub fn concurrent_execute_csv_file(
    concurrency: Option<usize>,
    path: impl AsRef<Path>,
    ledger: &Ledger,
    policy: Policy,
) -> Result<(), ExecError> {
    let mut f = std::fs::File::open(path)?;
    concurrent_execute_csv(concurrency, &mut f, ledger, policy)
}

pub fn concurrent_execute_csv(
    concurrency: Option<usize>,
    rd: impl std::io::Read,
    ledger: &Ledger,
    policy: Policy,
) -> Result<(), ExecError> {
    let mut ch: Vec<Sender<TxRequest>> = Vec::new();
    let wg = WaitGroup::new();
    let concurrency = match concurrency {
        Some(n) if n > 0 => n,
        _ => std::thread::available_parallelism().unwrap().get(),
    };
    let (res_s, res_r) = unbounded::<ExecError>();
    for _ in 0..concurrency {
        let res_s = res_s.clone();
        let (msg_s, msg_r) = bounded(MSG_QUEUE_LENGTH);
        ch.push(msg_s);
        let ledger = ledger.clone();
        let wg = wg.clone();
        thread::spawn(move || {
            let mut acc = BasicAccountant::with_policy(ledger, policy);
            loop {
                use TxType::*;
                let res = match msg_r.recv() {
                    Ok(tx) => match tx.tx_type {
                        Deposit => acc.deposit(tx.client, tx.tx_id, tx.amount.unwrap()),
                        Withdrawal => acc.withdrawal(tx.client, tx.tx_id, tx.amount.unwrap()),
                        Dispute => acc.dispute(tx.client, tx.tx_id),
                        Resolve => acc.resolve(tx.client, tx.tx_id),
                        Chargeback => acc.chargeback(tx.client, tx.tx_id),
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
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .comment(Some(b'#'))
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(rd);
    for result in rdr.deserialize() {
        let r: TxRequest = result?;
        use TxType::*;
        let wkr = shard_it(r.client.0 as u32, concurrency);
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

fn shard_it(index: u32, concurrency: usize) -> usize {
    let r = (((index + 1013904223) as u64) * 1664525) as u32;
    (r as usize * concurrency) >> 32
}

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
            Some(path) => sled::Config::default().path(path).open().and_then(|db| {
                db.clear()?;
                Ok(db)
            }),
            None => sled::Config::default().temporary(true).open(),
        }
        .map(|db| Ledger(db))
    }
    pub fn new() -> sled::Result<Ledger> {
        Self::new_empty(None)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Rec<K, V> {
    k: K,
    v: V,
}
type AccRec = Rec<Client, Account>;
type TxRec = Rec<TxId, Transaction>;

impl<'q> LedgerTrait<'q> for Ledger {
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
    fn accounts(&'q self) -> Box<dyn Iterator<Item = IterResult<(Client, Account)>> + 'q> {
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
    fn transactions(&'q self) -> Box<dyn Iterator<Item = IterResult<(TxId, Transaction)>> + 'q> {
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
