use clap::Parser;
use std::path::Path;
use toybank::basic::{Ledger as BasicLedger,Accountant};
use toybank::advanced::{Ledger as SledLedger,concurrent_execute_csv_file};
use toybank::common::{Accountant as _,Policy};
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
        _ => |x| match x {
            Some(x) => SledLedger::open(x),
            _ => SledLedger::new()
        }};
    match (args.ledger,args.concurrency) {
        (n, Some(cc)) if cc > 1 => {
            let ledger = open_sled(n).map_err(|e| ExecError::StringError(e.to_string()))?;
            concurrent_execute_csv_file(Some(cc), path, &ledger, policy)?;
            dump_accounts(std::io::stdout(), &ledger)
        }
        (Some(name), _) => {
            let ledger = open_sled(Some(name)).map_err(|e| ExecError::StringError(e.to_string()))?;
            let mut accountant = Accountant::with_policy(ledger, policy);
            execute_csv_file(path, &mut accountant)?;
            dump_accounts(std::io::stdout(), accountant.ledger())
        }
        _ => {
            let mut accountant = Accountant::with_policy(BasicLedger::default(), policy);
            execute_csv_file(path, &mut accountant)?;
            dump_accounts(std::io::stdout(), accountant.ledger())
        }
    }
}
