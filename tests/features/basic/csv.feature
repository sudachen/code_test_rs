Feature: Csv Processing

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
