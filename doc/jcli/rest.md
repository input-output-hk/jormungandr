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

```json
{
blockRecvCnt: 7,  # Blocks received by node
txRecvCnt: 90,    # Transactions received by node
uptime: 2101      # Node uptitme in seconds
}
```

## Whole UTXO

Fetches whole UTXO

```
jcli rest v0 utxo get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)


YAML printed on success

```
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
