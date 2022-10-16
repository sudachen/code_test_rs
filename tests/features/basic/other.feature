Feature: General Cases

  Scenario: Try to deposit funds
    Given new ledger
    When tx 1 deposit 10.1 to 1
    Then account 1 has total 10.1 available 10.1 held 0
    When tx 2 deposit 1.03 to 1
    Then account 1 has total 11.13 available 11.13 held 0

  Scenario: Dispute and Resolve
    Given new ledger
    When tx 1 deposit 1.1 to 1
    And dispute 1 for 1
    Then account 1 has total 1.1 available 0 held 1.1
    When resolve 1 for 1
    Then account 1 has total 1.1 available 1.1 held=0

  Scenario: Dispute and Chargeback
    Given new ledger
    When tx 1 deposit 1.1 to 1
    And dispute 1 for 1
    Then account 1 has total 1.1 available 0 held 1.1
    When chargeback 1 for 1
    Then account 1 has total 0 available 0 held=0
    And account 1 is locked

  Scenario: Double deposit
    Given new ledger
    When tx 1 deposit 1.1 to 1
    And tx 1 deposit 15 to 1 ignored
    Then account 1 has total 1.1 available 1.1 held 0

  Scenario: Double withdrawal
    Given new ledger
    When tx 1 deposit 1.1 to 1
    And tx 2 withdrawal 1 from 1
    And tx 2 withdrawal 1 from 1 ignored
    Then account 1 has total 0.1 available 0.1 held 0

  Scenario: Dispute/resolve/chargeback a withdrawal transaction
    Given new ledger
    When tx 1 deposit 1.1 to 1
    And tx 2 withdrawal 0.1 from 1
    And dispute 2 for 1 rejected
    And resolve 2 for 1 rejected
    And chargeback 2 for 1 rejected
    Then account 1 has total 1 available 1 held 0

  Scenario: Double dispute
    Given new ledger
    When tx 1 deposit 1.1 to 1
    And dispute 1 for 1
    And dispute 1 for 1 ignored
    Then account 1 has total 1.1 available 0 held 1.1