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


JSON printed on success

```
{
  "blockRecvCnt": 7,  // Blocks received by node
  "txRecvCnt": 90,    // Transactions received by node
  "uptime": 2101      // Node uptitme in seconds
}
```
