Feature: Advanced Csv Processing
  Scenario: a little more complex flow
    Given new ledger tests/test.ledger
      When execute csv
      """
      type,       client, tx, amount
      deposit,    1,      1,  1.0
      # 1 -> 1.0/0/1.0/false
      deposit,    2,      2,  2.0
      # 2 -> 2.0/0/2.0/false
      deposit,    3,      3,  3.0
      # 3 -> 3.0/0/3.0/false
      withdrawal, 1,      4,  1.1
      # rejected
      withdrawal, 2,      5,  1.1111
      # 2 -> 0.8889/0/0.8889/false
      dispute,    1,      4,
      # 1 -> 0/1.0/1.0/false
      resolve,    1,      3
      # rejected
      resolve,    1,      4
      # 1 -> 1.0/0/1.0/false
      dispute,    1,      4
      # rejected
      dispute,    2,      2
      # rejected
      deposit,    2,      5, 4.1111
      # rejected
      deposit,    2,      6, 4.1111
      # 2 -> 5.0/0/5.0/false
      dispute,    2,      2
      # 2 -> 3.0/2.0/5.0/false
      chargeback, 2,      2
      # 2 -> 3.0/0/3.0/true
      """
      Then validate accounts
      """
      client,     available,  held, total,  locked
      1,          1.0,        0,    1.0,    false
      2,          3.0,        0,    3.0,    true
      3,          3.0,        0,    3.0,    false
      """

  Scenario: reuse ledger
    Given open ledger tests/test.ledger
      Then validate accounts
        """
        client,     available,  held, total,  locked
        1,          1.0,        0,    1.0,    false
        2,          3.0,        0,    3.0,    true
        3,          3.0,        0,    3.0,    false
        """
