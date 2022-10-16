mod suite;
use toybank::basic::Accountant;

struct TheFactory;

impl suite::Factory for TheFactory {
    fn open(_: Option<String>) -> suite::DynBank {
        return suite::dyn_wrap(Accountant::default());
    }
    fn new(_: Option<String>) -> suite::DynBank {
        return suite::dyn_wrap(Accountant::default());
    }
}

#[test]
fn test() {
    suite::succeeded_with::<TheFactory>("tests/features/basic");
}
