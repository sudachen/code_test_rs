mod suite;
use toybank::{advanced::SledLedger, common::Policy};

struct TheFactory;

impl suite::Factory for TheFactory {
    fn open(name: String, policy: Policy) -> suite::Dyna {
        Box::new(SledLedger::open(name, policy).unwrap())
    }
    fn new(name: Option<String>, policy: Policy) -> suite::Dyna {
        Box::new(SledLedger::new_empty(name, policy).unwrap())
    }
}

#[test]
fn test() {
    suite::succeeded_with::<TheFactory>("tests/features/basic");
    suite::succeeded_with::<TheFactory>("tests/features/advanced");
}
