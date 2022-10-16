Feature: Csv Processing

  Rule: allow negative balance for dispute
    Scenario: dispute with insufficient balance
      Given new ledger
      When execute csv
        """
        type,       client, tx, amount
        deposit,    1,      1,  1.0
        withdrawal, 1,      2,  0.1
        dispute,    1,      1,
        chargeback, 1,      1,
        """
      Then validate accounts
        """
        client,     available,  held, total,  locked
        1,          -0.1,        0,   -0.1,   true
        """

  Rule: default
    Scenario: predefined test
      Given new ledger
      When execute csv
        """
        type,       client, tx, amount
        deposit,    1,      1,  1.0
        deposit,    2,      2,  2.0
        deposit,    1,      3,  2.0
        withdrawal, 1,      4,  1.5
        withdrawal, 2,      5,  3.0
        """
      Then validate accounts
        """
        client,     available,  held, total,  locked
        1,          1.5,        0,    1.5,    false
        2,          2,          0,    2,      false
        """

    Scenario: a little more complex flow
      Given new ledger
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
