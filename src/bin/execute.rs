use clap::Parser;
use std::path::Path;
use toybank::advanced::{concurrent_execute_csv_file, SledLedger};
use toybank::basic::HashLedger;
use toybank::common::Policy;
use toybank::libcsv::{dump_accounts, execute_csv_file, ExecError};

#[derive(Parser, Default, Debug)]
struct Arguments {
    /// CSV file containing transactions
    input_file: String,

    /// Level of transactions processing concurrency
    #[clap(short = 'p')]
    concurrency: Option<usize>,

    /// Allow negative balance for disputes
    #[clap(short = 'n')]
    allow_negative_dispute: bool,

    /// Persistent ledger name
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
    let open_sled = match args.drop_on_start {
        true => SledLedger::new_empty,
        _ => |x, policy| match x {
            Some(x) => SledLedger::open(x, policy),
            _ => SledLedger::new_empty(None, policy),
        },
    };
    match (args.ledger, args.concurrency) {
        (n, Some(cc)) if cc > 1 => {
            let ledger = open_sled(n, policy).map_err(|e| ExecError::StringError(e.to_string()))?;
            concurrent_execute_csv_file(Some(cc), path, ledger.clone())?;
            dump_accounts(std::io::stdout(), &ledger)
        }
        (Some(name), _) => {
            let mut ledger =
                open_sled(Some(name), policy).map_err(|e| ExecError::StringError(e.to_string()))?;
            execute_csv_file(path, &mut ledger)?;
            dump_accounts(std::io::stdout(), &ledger)
        }
        _ => {
            let mut ledger = HashLedger::with_policy(policy);
            execute_csv_file(path, &mut ledger)?;
            dump_accounts(std::io::stdout(), &ledger)
        }
    }
}
