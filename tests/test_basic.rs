mod suite;
use toybank::{basic::HashLedger, common::Policy};

struct TheFactory;

impl suite::Factory for TheFactory {
    fn open(_: String, _: Policy) -> suite::Dyna {
        panic!("open is not implemented for basic::Accountant using basic::Ledger");
    }
    fn new(_: Option<String>, policy: Policy) -> suite::Dyna {
        Box::new(HashLedger::with_policy(policy))
    }
}

#[test]
fn test() {
    suite::succeeded_with::<TheFactory>("tests/features/basic");
}
