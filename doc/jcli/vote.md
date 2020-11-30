# Voting

Jormungandr supports decentralized voting with privacy features.

The voting process is controlled by a committee whose private keys can be used
to decrypt and certify the tally.

## Creating committee keys

TBA

## Casting votes

TBA

## Tallying

### Public vote plan

To tally public votes, a single committee member is sufficient.
In the example below, the file `committee.sk` contains the committee member's
private key in bech32 format, and `block0.bin` contains the genesis block of
the voting chain.

```sh
genesis_block_hash=$(jcli genesis hash < block0.bin)
vote_plan_id=$(jcli rest v0 vote active plans get --output-format json|jq '.[0].id')
committee_addr=$(jcli address account $(jcli key to-public < committee.sk))
committee_addr_counter=$(jcli rest v0 account get "$committee_addr" --output-format json|jq .counter)
jcli certificate new vote-tally --vote-plan-id "$vote_plan_id" --output vote-tally.certificate
jcli transaction new --staging vote-tally.staging
jcli transaction add-account "$committee_addr" 0 --staging vote-tally.staging
jcli transaction add-certificate $(< vote-tally.certificate) --staging vote-tally.staging
jcli transaction finalize --staging vote-tally.staging
jcli transaction data-for-witness --staging vote-tally.staging > vote-tally.witness-data
jcli transaction make-witness --genesis-block-hash "$genesis_block_hash" --type account --account-spending-counter "$committee_addr_counter" $(< vote-tally.witness-data) vote-tally.witness committee.sk
jcli transaction add-witness --staging vote-tally.staging vote-tally.witness
jcli transaction seal --staging vote-tally.staging
jcli transaction auth --staging vote-tally.staging --key committee.sk
jcli transaction to-message --staging vote-tally.staging > vote-tally.fragment
jcli rest v0 message post --file vote-tally.fragment
```