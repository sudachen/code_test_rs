use std::path::Path;
use clap::Parser;
use toybank::basic::Accountant;
use toybank::libcsv::{ExecError,execute_csv_file,dump_accounts};

#[derive(Parser, Default, Debug)]
struct Arguments {
    input_file: String,
    #[clap(short = 'p')]
    concurrency: Option<usize>,
}

fn doit_by_default(path: &Path) -> Result<(),ExecError> {
    let mut bank = Accountant::default();
    execute_csv_file(path, &mut bank)?;
    dump_accounts(std::io::stdout(), &mut bank)
}

fn main() -> Result<(),ExecError>{
    let args = Arguments::parse();
    doit_by_default(Path::new(&args.input_file))
}
