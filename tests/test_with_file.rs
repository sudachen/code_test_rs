mod suite;
use toybank::advanced::Ledger;
use toybank::basic::Accountant;
use toybank::common::Policy;

struct TheFactory;

impl suite::Factory for TheFactory {
    fn open(name: String, policy: Policy) -> suite::Dyna {
        let ledger = Ledger::open(name).unwrap();
        return suite::dyna_make(Accountant::with_policy(ledger, policy));
    }
    fn new(name: Option<String>, policy: Policy) -> suite::Dyna {
        let ledger = Ledger::new_empty(name).unwrap();
        return suite::dyna_make(Accountant::with_policy(ledger, policy));
    }
}

#[test]
fn test() {
    suite::succeeded_with::<TheFactory>("tests/features/basic");
    suite::succeeded_with::<TheFactory>("tests/features/advanced");
}
