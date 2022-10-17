use clap::Parser;
use std::path::Path;
use toybank::basic::{Accountant, Ledger};
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
}

fn doit_by_default(path: &Path, policy: Policy) -> Result<(), ExecError> {
    let mut bank = Accountant::with_policy(Ledger::default(), policy);
    execute_csv_file(path, &mut bank)?;
    dump_accounts(std::io::stdout(), &mut bank)
}

fn main() -> Result<(), ExecError> {
    let args = Arguments::parse();
    let policy = Policy {
        allow_negative_balance_for_dispute: args.allow_negative_dispute,
        ..Default::default()
    };
    doit_by_default(Path::new(&args.input_file), policy)
}
