#![cfg(test)]

use cucumber::{
    codegen::Regex,
    { gherkin::Step, given, then, when, World as _ }
};
use futures::{ self, FutureExt as _ };
use rust_decimal::Decimal;
use std::{ default::Default, fmt::Debug, marker::PhantomData};
use toybank::common::{Ledger, Policy, TxError};

pub type Dyna = Box<dyn Ledger>;

pub trait Factory {
    fn open(ledger: String, policy: Policy) -> Dyna;
    fn new(ledger: Option<String>, policy: Policy) -> Dyna;
}

pub trait CustomTest {
    fn new_ledger(&mut self, _leger: Option<String>) {
        panic!("uninitialized")
    }
    fn open_ledger(&mut self, _leger: String) {
        panic!("uninitialized")
    }
    fn dyna(&mut self) -> &mut dyn Ledger {
        panic!("uninitialized")
    }
}

#[derive(Default)]
struct CustomTestImpl<F: Factory>(Option<Dyna>, Policy, PhantomData<F>);

impl<F: Factory> CustomTest for CustomTestImpl<F> {
    fn new_ledger(&mut self, leger: Option<String>) {
        self.0 = Some(F::new(leger, self.1))
    }

    fn open_ledger(&mut self, leger: String) {
        self.0 = Some(F::open(leger, self.1))
    }

    fn dyna(&mut self) -> &mut dyn Ledger {
        if let Some(x) = &mut self.0 {
            return &mut **x;
        }
        panic!("ledger is not selected")
    }
}

#[derive(cucumber::World)]
struct Test(Box<dyn CustomTest>);

impl Debug for Test {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Test")
    }
}

struct UninitCustomTest;
impl CustomTest for UninitCustomTest {}
impl Default for Test {
    fn default() -> Self {
        Self(Box::new(UninitCustomTest {}))
    }
}

fn err(status: Result<(), TxError>, j: String) -> Result<(), String> {
    let j = j.trim();
    match status {
        Ok(_) => {
            if j == "" {
                Ok(())
            } else {
                Err(format!("succeeded but must be {j}").into())
            }
        }
        Err(TxError::Rejected(e)) => {
            if j == "rejected" {
                Ok(())
            } else {
                Err(format!("rejected: {e}"))
            }
        }
        Err(TxError::Ignored(e)) => {
            if j == "ignored" {
                Ok(())
            } else {
                Err(format!("ignored: {e}"))
            }
        }
        Err(TxError::IOError(e)) => Err(format!("IoError: {e}")),
        Err(TxError::StringError(e)) => Err(format!("{e}")),
        Err(TxError::Empty) => Err("empty".into()),
    }
}

#[given(regex = r"new\s+ledger(\s+[^\s]+)?")]
fn new_leger(w: &mut Test, name: String) {
    match name.trim() {
        "" => w.0.new_ledger(None),
        path => w.0.new_ledger(Some(path.into())),
    }
}

#[given(regex = r"existing\s+ledger(\s+[^\s]+)?")]
fn open_leger(w: &mut Test, name: String) {
    w.0.open_ledger(name.trim().into())
}

#[when(regex = r"tx\s+(\d+)\s+deposit\s+(\d*\.?\d+)\s+to\s+(\d+)(\s+rejected|\s+ignored)?")]
fn deposit(w: &mut Test, tx: u32, a: String, c: u32, j: String) {
    let amount = Decimal::from_str_exact(a.as_str()).unwrap();
    let status = w.0.dyna().deposit(c.into(), tx.into(), amount);
    assert_eq!(err(status, j), Ok(()))
}

#[when(regex = r"tx\s+(\d+)\s+withdrawal\s+(\d*\.?\d+)\s+from\s+(\d+)(\s+rejected|\s+ignored)?")]
fn withdrawal(w: &mut Test, tx: u32, a: String, c: u32, j: String) {
    let amount = Decimal::from_str_exact(a.as_str()).unwrap();
    let status = w.0.dyna().withdrawal(c.into(), tx.into(), amount);
    assert_eq!(err(status, j), Ok(()))
}

#[when(regex = r"dispute\s+(\d+)\s+for\s+(\d+)(\s+rejected|\s+ignored)?")]
fn dispute(w: &mut Test, tx: u32, c: u32, j: String) {
    let status = w.0.dyna().dispute(c.into(), tx.into());
    assert_eq!(err(status, j), Ok(()))
}

#[when(regex = r"resolve\s+(\d+)\s+for\s+(\d+)(\s+rejected|\s+ignored)?")]
fn resolve(w: &mut Test, tx: u32, c: u32, j: String) {
    let status = w.0.dyna().resolve(c.into(), tx.into());
    assert_eq!(err(status, j), Ok(()))
}

#[when(regex = r"chargeback\s+(\d+)\s+for\s+(\d+)(\s+rejected|\s+ignored)?")]
fn chargeback(w: &mut Test, tx: u32, c: u32, j: String) {
    let status = w.0.dyna().chargeback(c.into(), tx.into());
    assert_eq!(err(status, j), Ok(()))
}

#[then(
    regex = r"account\s+(\d+)\s+has\s+total[=\s](\d*\.?\d+)\s+available[=\s](\d*\.?\d+)\s+held[=\s](\d*\.?\d+)"
)]
fn account_has(w: &mut Test, c: u32, t: String, a: String, h: String) {
    let available = Decimal::from_str_exact(a.as_str()).unwrap();
    let total = Decimal::from_str_exact(t.as_str()).unwrap();
    let held = Decimal::from_str_exact(h.as_str()).unwrap();
    let acc = w.0.dyna().get_account(c.into()).unwrap();
    assert!(acc.is_some());
    assert_eq!(acc.unwrap().available, available);
    assert_eq!(acc.unwrap().total, total);
    assert_eq!(acc.unwrap().held, held);
}

#[then(regex = r"account\s+(\d+)\s+is\s+locked")]
fn account_is_locked(w: &mut Test, c: u32) {
    let acc = w.0.dyna().get_account(c.into()).unwrap();
    assert!(acc.is_some());
    assert!(acc.unwrap().locked);
}

#[when("execute csv")]
fn execute_csv(w: &mut Test, step: &Step) {
    let x = step.docstring.clone().unwrap();
    if let Err(e) = toybank::libcsv::execute_csv(std::io::Cursor::new(x.as_bytes()), w.0.dyna()) {
        panic!("error occured: {e}")
    }
}

#[then("validate accounts")]
fn validate_accounts(w: &mut Test, step: &Step) {
    let x = step.docstring.clone().unwrap();
    if let Err(e) =
        toybank::libcsv::validate_accounts(std::io::Cursor::new(x.as_bytes()), w.0.dyna())
    {
        panic!("error occured: {e}")
    }
}

// Since CLion IDE (with JetBrains Rust plugin) calls
//    `cargo test` with additional params I really don't need,
//    this dirty trick allows to skips all those params
// yeh, it's possible to use libtest writer, but the output then becomes ugly and useless
#[derive(clap::Args)]
//#[command(allow_external_subcommands(true))]
struct CustomCli {
    test_name: Option<String>,
    #[arg(long)]
    format: Option<String>,
    #[arg(long)]
    exact: bool,
    #[arg(short = 'Z')]
    z: Option<String>,
    #[arg(long = "show-output")]
    so: bool,
}

#[allow(dead_code)]
pub fn run_with<F: Factory + 'static>(features: &str) -> usize {
    let cli = cucumber::cli::Opts::<_, _, _, CustomCli>::parsed();
    let t = Test::cucumber().with_cli(cli).before(move |_, r, _, w| {
        async move {
            let mut policy: Policy = Default::default();
            if let Some(rule) = r {
                let rx = Regex::new(r"(allow|deny) negative balance for dispute").unwrap();
                if let Some(x) = rx.captures(rule.name.as_str()) {
                    match &x[1] {
                        "allow" => policy.allow_negative_balance_for_dispute = true,
                        "deny" => policy.allow_negative_balance_for_dispute = false,
                        _ => (),
                    }
                }
            }
            w.0 = Box::new(CustomTestImpl::<F>(None, policy, PhantomData));
        }
        .boxed_local()
    });
    let res = futures::executor::block_on(t.run(features));
    res.scenarios.failed
}

#[allow(dead_code)]
pub fn succeeded_with<F: Factory + 'static>(features: &str) {
    let n = run_with::<F>(features);
    assert_eq!(n, 0, "{n} failed scenarios");
}
