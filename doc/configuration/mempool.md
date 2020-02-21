When running an active node (BFT leader or stake pool) it is interesting to be
able to make choices on how to manage the pending transactions: how long to keep
them, how to prioritize them etc.

The `mempool` field in your node config file is not mandatory, by default it is set
as follow:

```yaml
mempool:
    pool_max_entries: 10000
    log_max_entries: 100000
```

* `pool_max_entries`: (optional, default is 10000). Set a maximum size of the mempool
* `log_max_entries`: (optional, default is 100000). Set a maximum size of fragment logs
