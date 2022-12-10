[![codecov](https://codecov.io/gh/sudachen/code_test_rs/branch/master/graph/badge.svg?token=Z03QYMSP1J)](https://codecov.io/gh/sudachen/code_test_rs) 
[![](https://github.com/sudachen/code_test_rs/actions/workflows/main.yml/badge.svg)](https://github.com/sudachen/code_test_rs/actions/workflows/main.yml?query=actor%3Aborsborg+branch%3Astaging+is%3Asuccess)


This was originally my "Rust Coding Test" solution for some company that I won't name here.

In a few words, the original problem sounds like: 
"you need to implement a transaction processor
which takes a CSV file and prints resulting accounts state".

![image](https://user-images.githubusercontent.com/1428/206829287-80207d29-1407-4f1b-9d40-4f772fa290e5.png)

So it must be processed as:

![image](https://user-images.githubusercontent.com/1428/206829330-a088893d-a38d-490f-8a11-7e0ec2acbc60.png)

To be sure of my solution, I added some cucumber scripts to test the behavior of the solution a little more solid. 

![image](https://user-images.githubusercontent.com/1428/206829317-6868db17-b284-4b7e-a202-9a2afc8a0564.png)

The code is divided into the following modules:
- The module [common](src/common.rs) defining constants, errors, traits Ledger, etc.
- The module [basic](src/basic.rs) defining basic implementation of Ledger with HashMap.
- The module [libcsv](src/libcsv.rs) defining csv processing functions.

The main program [execute](/src/bin/execute.rs) is in the src/bin subdirectory. 
It uses basic implementation of Ledger to process transactions from a CSV file.  

