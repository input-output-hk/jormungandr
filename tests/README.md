## Coverage Status

| Modules     | Feature             | Number of test cases | Automated | Coverage         |
|:------------|:--------------------|:---------------------|:----------|:-----------------|
| JCLI        |                     |                      |           |                  |
|             | address             | 17                   | 1         | 6 %              |
|             | certificate         | 12                   | 0         | 0 %              |
|             | debug               | 3                    | 0         | 0 %              |
|             | genesis             | 34                   | 1         | 3 %              |
|             | key                 | 14                   | 2         | 14 %             |
|             | rest                | 20                   | 3         | 15 %             |
|             | transaction         | 6                    | 1         | 17 %             |
| Jormungandr |                     |                      |           |                  |
|             | startup             | 7                    | 1         | 14  %            |
|             | transaction         | 14                   | 3         | 21  %            |
|             | configuration       | 21                   | 1         | 5   %            |
|             | node communications | 8                    | 0         | 0   %            |
| Summary     |                     | 156                  | 13        | **8 %**          |

### Coverage History 

![Alt text](images/automation_coverage.PNG?raw=true "Coverage History")

## Automation Status

### Automation Report
| Test                                                                  | Status | Desc         | Bug ID |
|:----------------------------------------------------------------------|:-------|:-------------|:-------|
| test_delegation_address_is_the_same_as_public                         | ok     |              |        |
| test_account_address_made_of_ed25519_extended_key                     | failed | Sev 4        | #306   |
| test_utxo_address_made_of_ed25519_extended_key                        | ok     |              |        |
| test_account_address_made_of_incorrect_ed25519_extended_key           | ok     |              |        |
| test_utxo_address_made_of_incorrect_ed25519_extended_key              | failed | Sev 4        | #306   |
| test_delegation_address_made_of_random_string                         | failed | Sev 4        | #306   |
| test_delegation_address_made_of_ed25519_extended_seed_key             | ok     |              |        |
| test_delegation_address_made_of_incorrect_public_ed25519_extended_key | failed | Sev 4        | #306   |
| test_genesis_block_is_built_from_corect_yaml                          | ok     |              |        |
| test_ed25510bip32_key_generation                                      | ok     |              |        |
| test_ed25519_key_generation                                           | ok     |              |        |
| test_ed25519extended_key_generation                                   | ok     |              |        |
| test_curve25519_2hashdh_key_generation                                | ok     |              |        |
| test_fake_mm_key_generation                                           | ok     |              |        |
| test_key_to_public                                                    | ok     |              |        |
| test_key_to_public_invalid_key                                        | ok     |              |        |
| test_key_with_seed_generation                                         | ok     |              |        |
| test_key_with_seed_with_unknown_symbol_generation                     | ok     |              |        |
| test_key_with_too_long_seed_generation                                | ok     |              |        |
| test_key_with_too_short_seed_generation                               | ok     |              |        |
| test_unknown_key_type_generation                                      | ok     |              |        |
| test_private_key_to_public_key                                        | ok     |              |        |
| test_key_from_and_to_bytes                                            | ok     |              |        |
| test_correct_utxos_are_read_from_node                                 | ok     |              |        |
| test_unbalanced_output_utxo_transation_is_rejected                    | ok     |              |        |
| test_utxo_transation_with_more_than_one_witness_per_input_is_rejected | ok     |              |        |
| test_correct_utxo_transaction_is_accepted_by_node                     | ok     |              |        |
| test_jormungandr_node_starts_successfully                             | ok     |              |        |

### Automation Passrate 

![Alt text](images/automation_passrate.PNG?raw=true "Automation Passrate")


