# REST

Jormungandr comes with a CLI client for manual communication with nodes over HTTP.

## Conventions

Many CLI commands have common arguments:

- `-h <addr>` or `--host <addr>` - Node API address. Must always have `http://` or
`https://` prefix. E.g. `-h http://127.0.0.1`, `--host https://node.com:8443/cardano/api`
- `--debug` - Print additional debug information to stderr.
The output format is intentionally undocumented and unstable
- `--output-format <format>` - Format of output data. Possible values: json, yaml, default yaml.
Any other value is treated as a custom format using values from output data structure.
Syntax is Go text template: https://golang.org/pkg/text/template/.

## Node stats

Fetches node stats

```
jcli rest v0 node stats get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)


YAML printed on success

```yaml
---
blockRecvCnt: 7 # Blocks received by node
txRecvCnt: 90   # Transactions received by node
uptime: 2101    # Node uptitme in seconds
```

## Whole UTXO

Fetches whole UTXO

```
jcli rest v0 utxo get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)


YAML printed on success

```yaml
---
- in_idx: 0                                                                 # input index
  in_txid: 50f21ac6bd3f57f231c4bf9c5fff7c45e2529c4dffed68f92410dbf7647541f1 # input transaction hash in hex
  out_addr: ca1qvqsyqcyq5rqwzqfpg9scrgwpugpzysnzs23v9ccrydpk8qarc0jqxuzx4s  # output address in bech32
  out_value: 999999999                                                      # output value
```

## Post transaction

Posts a signed, hex-encoded transaction

```
jcli rest v0 message post <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- -f --file <file_path> - File containing hex-encoded transaction.
If not provided, transaction will be read from stdin.


Fragment Id is printed on success (which can help finding transaction status using get message log command)

```
50f21ac6bd3f57f231c4bf9c5fff7c45e2529c4dffed68f92410dbf7647541f1
```

## Get message log

Get the node's logs on the message pool. This will provide information on pending transaction,
rejected transaction and or when a transaction has been added in a block

```
jcli rest v0 message logs <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)

YAML printed on success

```yaml
---
- fragment_id: 7db6f91f3c92c0aef7b3dd497e9ea275229d2ab4dba6a1b30ce6b32db9c9c3b2 # hex-encoded fragment ID
  last_updated_at: 	2019-06-02T16:20:26.201000000Z                              # RFC3339 timestamp of last fragment status change
  received_at: 2019-06-02T16:20:26.201000000Z                                   # RFC3339 timestamp of fragment receivement
  received_from: Network,                                                       # how fragment was received
  status: Pending,                                                              # fragment status
```

`received_from` can be one of:

```yaml
received_from: Rest     # fragment was received from node's REST API
```

```yaml
received_from: Network  # fragment was received from the network
```

`status` can be one of:

```yaml
status: Pending                 # fragment is pending
```

```yaml
status:
  Rejected:                     # fragment was rejected
    reason: reason of rejection # cause
```

```yaml
status:                         # fragment was included in a block
  InABlock: "6637.3"            # block epoch and slot ID formed as <epoch>.<slot_id>
```

## Blockchain tip

Retrieves a hex-encoded ID of the blockchain tip

```
jcli rest v0 tip get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)

## Get block

Retrieves a hex-encoded block with given ID

```
jcli rest v0 block <block_id> get <options>
```

<block_id> - hex-encoded block ID

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)

## Get next block ID

Retrieves a list of hex-encoded IDs of descendants of block with given ID.
Every list element is in separate line. The IDs are sorted from closest to farthest.

```
jcli rest v0 block <block_id> next-id get <options>
```

<block_id> - hex-encoded block ID

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- -c --count <count> - Maximum number of IDs, must be between 1 and 100, default 1

## Get account state

Get account state

```
jcli rest v0 account get <account-id> <options>
```

<account-id> - ID of an account, bech32-encoded

The options are
- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)

YAML printed on success

```yaml
---
counter: 1
delegation: c780f14f9782770014d8bcd514b1bc664653d15f73a7158254730c6e1aa9f356
value: 990
```

* `value` is the current balance of the account;
* `counter` is the number of transactions performed using this account
  this is useful to know when signing new transactions;
* `delegation` is the Stake Pool Identifier the account is delegating to.
  it is possible this value is not set if there is no delegation certificate
  sent associated to this account.

## Node settings

Fetches node settings

```
jcli rest v0 settings get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)


YAML printed on success

```yaml
---
block0Hash: 8d94ecfcc9a566f492e6335858db645691f628b012bed4ac2b1338b5690355a7  # block 0 hash of
block0Time: "2019-07-09T12:32:51+00:00"         # block 0 creation time of
consensusVersion: bft                           # currently used consensus
currSlotStartTime: "2019-07-09T12:55:11+00:00"  # current slot start time
fees:                                           # transaction fee configuration
  certificate: 4                                # fee per certificate
  coefficient: 1                                # fee per every input and output
  constant: 2                                   # fee per transaction
maxTxsPerBlock: 100                             # maximum number of transactions in block
```

## Node shutdown

Node shutdown

```
jcli rest v0 shutdown get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)

## Get leaders

Fetches list of leader IDs

```
jcli rest v0 leaders get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)


YAML printed on success

```yaml
---
- 1 # list of leader IDs
- 2
```

## Register leader

Register new leader and get its ID

```
jcli rest v0 leaders post <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)
-f, --file <file> - File containing YAML with leader secret. It must have the same format as secret YAML passed to Jormungandr as --secret. If not provided, YAML will be read from stdin.

On success created leader ID is printed

```
3
```

## Delete leader

Delete leader with given ID

```
jcli rest v0 leaders delete <id> <options>
```

<id> - ID of deleted leader

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)

## Get leadership logs

Fetches leadership logs

```
jcli rest v0 leaders logs get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)


YAML printed on success

```yaml
---
- created_at_time: "2019-08-19T12:25:00.417263555+00:00"
  enclave_leader_id: 1
  finished_at_time: "2019-08-19T23:19:05.010113333+00:00"
  scheduled_at_date: "0.3923"
  scheduled_at_time: "2019-08-19T23:18:35+00:00"
  wake_at_time: "2019-08-19T23:18:35.001254555+00:00"
```

## Get stake pools

Fetches list of stake pool IDs

```
jcli rest v0 stake-pools get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)


YAML printed on success

```yaml
---
- 5cf03f333f37eb7b987dbc9017b8a928287a3d77d086cd93cd9ad05bcba7e60f # list of stake pool IDs
- 3815602c096fcbb91072f419c296c3dfe1f730e0f446a9bd2553145688e75615
```

## Get stake distribution

Fetches stake information

```
jcli rest v0 stake get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)


YAML printed on success

```yaml
---
epoch: 228      # Epoch of last block
stake:
  dangling: 0 # Total value stored in accounts, but assigned to nonexistent pools
  pools:
    - - 5cf03f333f37eb7b987dbc9017b8a928287a3d77d086cd93cd9ad05bcba7e60f # stake pool ID
      - 1000000000000                                                    # staked value
    - - 3815602c096fcbb91072f419c296c3dfe1f730e0f446a9bd2553145688e75615 # stake pool ID
      - 1000000000000                                                    # staked value
  unassigned: 0 # Total value stored in accounts, but not assigned to any pool
```

## Network stats

Fetches network stats

```
jcli rest v0 network stats get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)
- --debug - see [conventions](#conventions)
- --output-format <format> - see [conventions](#conventions)


YAML printed on success

```yaml
---
  # hex-encoded node ID
- nodeId: 0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20
  # timestamp of when the connection was established
  establishedAt: "2019-10-14T06:24:12.010231281+00:00"
  # timestamp of last time block was received from node if ever (optional)
  lastBlockReceived: "2019-10-14T00:45:57.419496113+00:00"
  # timestamp of last time fragment was received from node if ever (optional)
  lastFragmentReceived: "2019-10-14T00:45:58.419496150+00:00"
  # timestamp of last time gossip was received from node if ever (optional)
  lastGossipReceived: "2019-10-14T00:45:59.419496188+00:00"
```
