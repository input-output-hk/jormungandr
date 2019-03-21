# REST client

Jormungandr comes with a CLI client for manual communication with nodes over HTTP.

## Convention

Many CLI commands have common arguments:

- `-h <addr>` or `--host <addr>` - Node API address. Must always have `http://` or
`https://` prefix. E.g. `-h http://127.0.0.1`, `--host https://node.com:8443/cardano/api`

## Node stats

Fetches node stats

```
jormungandr_cli rest v0 node stats get <options>
```

The options are

- -h <node_addr> - see [conventions](#conventions)


YAML printed on success

```
---
blockRecvCnt: 7,  # Blocks received by node
txRecvCnt: 90,    # Transactions received by node
uptime: 2101      # Node uptitme in seconds
}
```

## Whole UTXO

Fetches whole UTXO

```
jormungandr_cli rest v0 utxo get <options>
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
