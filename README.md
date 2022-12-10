[![codecov](https://codecov.io/gh/sudachen/code_test_rs/branch/master/graph/badge.svg?token=Z03QYMSP1J)](https://codecov.io/gh/sudachen/code_test_rs) 
[![](https://github.com/sudachen/code_test_rs/actions/workflows/main.yml/badge.svg)](https://github.com/sudachen/code_test_rs/actions/workflows/main.yml?query=actor%3Aborsborg+branch%3Astaging+is%3Asuccess)


It's my "Rust Coding Test" artifact. 
In a few words, the original problem sounds like this: 
"you need to implement a transaction processor
which takes a CSV file and prints resulting accounts state".

The CSV file is presented like:
```csv
type,       client, tx, amount
deposit,    1,      1,  1.0
deposit,    2,      2,  2.0
deposit,    3,      3,  3.0
deposit,    1,      4,  2.0
withdrawal, 1,      5,  0.5
dispute,    1,      1,
chargeback, 1,      1,
```
So it must be processed as:

```console
[sudachen/sudachen_code_test_rs.git|master]$ cargo run -- tests/test_tx_1.csv
    Finished dev [unoptimized + debuginfo] target(s) in 0.07s
     Running `target/debug/execute tests/test_tx_1.csv`
client,available,held,total,locked
1,1.5,0,1.5,true
3,3,0,3,false
2,2,0,2,false
```

The solution has a set of cucumber tests for specific cases. To run tests use:
```console
[sudachen/sudachen_code_test_rs.git|master] ../toybank$ cargo test
   Compiling toybank v0.1.0 (/Projects/kraken/toybank)
    Finished test [unoptimized + debuginfo] target(s) in 3.10s
     Running tests/test_basic.rs (target/debug/deps/test_basic-924d9e19eb32a4fb)

running 1 test
Feature: Csv Processing
  Rule: allow negative balance for dispute
    Scenario: dispute with insufficient balance
     ✔  Given new ledger
     ✔  When execute csv
     ✔  Then validate accounts
  Rule: default
    Scenario: predefined test
     ✔  Given new ledger
     ✔  When execute csv
     ✔  Then validate accounts
    Scenario: a little more complex flow
     ✔  Given new ledger
     ✔  When execute csv
     ✔  Then validate accounts
...
```

The code is divided into the following modules:
- The module [common](src/common.rs) defining constants, errors, traits Ledger, etc.
- The module [basic](src/basic.rs) defining basic implementation of Ledger with HashMap.
- The module [libcsv](src/libcsv.rs) defining csv processing functions.

The main program [execute](/src/bin/execute.rs) is in the src/bin subdirectory. 
It uses basic implementation of Ledger to process transactions from a CSV file.  

