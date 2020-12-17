# Voting

Jormungandr supports decentralized voting with privacy features.

The voting process is controlled by a committee whose private keys can be used
to decrypt and certify the tally.

## Creating committee keys

### Private
Please refer to `jcli votes committee --help` for help with the committee related cli operations and specification of arguments.

In this example we will be using 3 kind of keys for the private vote and tallying.

In order:

#### Committee communication key

```shell
jcli votes committee communication-key generate > ./comm.sk
```

We can get its public representation with:

```shell
jcli votes committee communication-key to-public --input ./comm.sk > ./comm.pk
```

#### Committee member key

```shell
crs=$(jcli vote crs generate)
jcli votes committee member-key generate --threshold 3 --crs "$crs" --index 0 --keys pk1 pk2 pk3 > ./member.sk
```
Where `pkX` are each of the committee communication public keys.
We can also easily get its public representation as before:

```shell
jcli votes committee member-key to-public --input ./member.sk ./member.pk
```


#### Vote encrypting key
This key (*public*) is the key **every vote** should be encrypted with.

```shell
jcli votes encrypting-key --eys mpk1 mpk2 mpkn > ./vote.pk
```

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

### Private

