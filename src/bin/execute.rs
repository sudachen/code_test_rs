use clap::Parser;
use std::path::Path;
use std::sync::{Arc, Mutex};
use toybank::advanced::{
    index_by_client, sharded_dump_accounts, sharded_execute_csv_file, SledLedger,
};
use toybank::basic::HashLedger;
use toybank::common::{Ledger, Policy};
use toybank::libcsv::{dump_accounts, execute_csv_file, ExecError};

#[derive(Parser, Default, Debug)]
struct Arguments {
    /// CSV file containing transactions
    input_file: String,

    /// Count of workers to process transactions, 0 means count of vCPUs
    #[clap(short = 'p')]
    concurrency: Option<usize>,

    /// Allow negative balance for disputes
    #[clap(short = 'n')]
    allow_negative_dispute: bool,

    /// Persistent ledger name, or `inmem` to use inmem SledDB, otherwise hashtable is used
    #[clap(long)]
    ledger: Option<String>,

    /// Drop ledger content on start)
    #[clap(long = "drop")]
    drop_on_start: bool,
}

fn main() -> Result<(), ExecError> {
    let args = Arguments::parse();
    let policy = Policy {
        allow_negative_balance_for_dispute: args.allow_negative_dispute,
        ..Default::default()
    };
    let path = Path::new(&args.input_file);
    let concurrency = match args.concurrency {
        Some(0) => std::thread::available_parallelism().unwrap().get(),
        Some(n) => n,
        None => 1,
    };
    match args.ledger {
        // SledDb
        Some(name) => {
            let mut ledger = if name == "inmem" {
                SledLedger::new_empty(None, policy)
            } else {
                match args.drop_on_start {
                    true => SledLedger::new_empty(Some(name), policy),
                    _ => SledLedger::open(name, policy),
                }
            }
            .map_err(|e| ExecError::StringError(e.to_string()))?;
            if concurrency > 1 {
                let sharding = ledger.sharding(concurrency);
                sharded_execute_csv_file(path, &sharding, index_by_client)
            } else {
                execute_csv_file(path, &mut ledger)
            }?;
            dump_accounts(std::io::stdout(), &ledger)
        }
        // HashMap
        None => {
            if concurrency > 1 {
                let sharding = (0..concurrency)
                    .map(|_| {
                        Arc::new(Mutex::new(HashLedger::with_policy(policy)))
                            as Arc<Mutex<dyn Ledger + Send>>
                    })
                    .collect();
                sharded_execute_csv_file(path, &sharding, index_by_client)?;
                sharded_dump_accounts(std::io::stdout(), &sharding, index_by_client)
            } else {
                let mut ledger = HashLedger::with_policy(policy);
                execute_csv_file(path, &mut ledger)?;
                dump_accounts(std::io::stdout(), &ledger)
            }
        }
    }
}
