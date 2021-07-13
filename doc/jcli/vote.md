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
jcli votes committee member-key generate --threshold 3 --crs "$crs" --index 0 --keys pk1 pk2 pk3 > ./member.sk
```
Where `pkX` are each of the committee communication public keys in bech32 format.
The order of the keys shall be the same for every member invoking the command,
and the `--index` parameter provides the 0-based index of the member this key
is generated for.
Note that **all committee members shall use the same CRS**.

We can also easily get its public representation as before:

```shell
jcli votes committee member-key to-public --input ./member.sk ./member.pk
```


#### Election public key
This key (*public*) is the key **every vote** should be encrypted with.

```shell
jcli votes election-key --keys mpk1 mpk2 mpk3 > ./vote.pk
```

Notice that we can always rebuild this key with the committee member public keys found
within the [voteplan certificate](#creating-a-vote-plan).

```shell
jcli rest v0 vote active plans > voteplan.json
```



## Creating a vote plan

We need to provide a vote plan definition file to generate a new voteplan certificate.
That file should be a `yaml` (or json) with the following format:
```yaml
{
  "payload_type": "private",
  "vote_start": {
    "epoch": 1,
    "slot_id": 0
  },
  "vote_end": {
    "epoch": 3,
    "slot_id": 0
  },
  "committee_end": {
    "epoch": 6,
    "slot_id": 0
  },
  "proposals": [
    {
      "external_id": "d7fa4e00e408751319c3bdb84e95fd0dcffb81107a2561e691c33c1ae635c2cd",
      "options": 3,
      "action": "off_chain"
    },
    ...
  ],
  "committee_member_public_keys": [
    "pk....",
  ]
}
```
Where:
* payload_type is either *public* or *private*
* commitee_public_keys is only needed for private voting, can be empty for public.

Then, we can generate the voteplan certificate with:

```shell
jcli certificate new vote-plan voteplan_def.json --output voteplan.certificate
```

## Casting votes

TBA

## Tallying

### Public vote plan

To tally public votes, a single committee member is sufficient.
In the example below, the file `committee.sk` contains the committee member's
private key in bech32 format, and `block0.bin` contains the genesis block of
the voting chain.

```shell
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
To tally private votes, all committee members are needed.
The process is similar to the public one, but we need to issue different certificates.

First, we need to retrieve vote plans info:
```shell
jcli rest v0 vote active plans > active_plans.json
```
If there is more than one vote plan in the file, we also need to provide the id of the vote plan we are interested in to following commands. We can get the id of the first vote plan with:
```shell
...
vote_plan_id=$(cat active_plans.json |jq '.[0].id')
...
```
Each committee member needs to generate their shares for the vote plan, which we will use later to decrypt the tally.

```shell
jcli votes tally decryption-shares --vote-plan active_plans.json --vote-plan-id $"vote_plan_id" --key member.sk --output-format json
```
Then, the committee members need to exchange their shares (only one full set of shares is needed).
Once all shares are available, we need to merge them in a single file with the following command (needed even if there is only one set of shares):

```shell
jcli votes tally merge-shares  share_file1 share_file2 ... > merged_shares.json
```


With the merged shares file, we are finally able to process the final tally result as follows:

```shell
jcli votes tally decrypt-results \
--vote-plan active_plans.json \
--vote-plan-id $"vote_plan_id" \
--shares merged_shares.json \
--threshold number_of_committee_members \
--output-format json > result.json
```
