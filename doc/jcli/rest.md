# REST

Jormungandr comes with a CLI client for manual communication with nodes over HTTP.

## Conventions

Many CLI commands have common arguments:

- `-h <addr>` or `--host <addr>` - Node API address. Must always have `http://` or
`https://` prefix. E.g. `-h http://127.0.0.1`, `--host https://node.com:8443/cardano/api`

## Node stats

Fetches node stats

```
jcli rest v0 node stats get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)


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
- -f --file <file_path> - File containing hex-encoded transaction.
If not provided, transaction will be read from stdin.

## Get message log

Get the node's logs on the message pool. This will provide information on pending transaction,
rejected transaction and or when a transaction has been added in a block

```
jcli rest v0 message logs <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)

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

## Get block

Retrieves a hex-encoded block with given ID

```
jcli rest v0 block <block_id> get <options>
```

<block_id> - hex-encoded block ID

The options are

- -h <node_addr> - see [conventions](#conventions)

## Get next block ID

Retrieves a list of hex-encoded IDs of descendants of block with given ID.
Every list element is in separate line. The IDs are sorted from closest to farthest.

```
jcli rest v0 block <block_id> next-id get <options>
```

<block_id> - hex-encoded block ID

The options are

- -h <node_addr> - see [conventions](#conventions)
- -c --count <count> - Maximum number of IDs, must be between 1 and 100, default 1

## Get account state

Get account state

```
jcli rest v0 account get <account-id> <options>
```

<account-id> - ID of an account, bech32-encoded

The options are
- -h <node_addr> - see [conventions](#conventions)

YAML printed on success

```yaml
---
value: 257357,    # current balance of account in lovelace
counter: 4,       # number of transactions performed using this account
```
