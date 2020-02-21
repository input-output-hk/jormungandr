When running an active node (BFT leader or stake pool) it is interesting to be
able to make choices on how to manage the pending transactions: how long to keep
them, how to prioritize them etc.

The `mempool` field in your node config file is not mandatory, by default it is set
as follow:

```yaml
mempool:
    pool_max_entries: 10000
    fragment_ttl: 30m
    log_max_entries: 100000
    garbage_collection_interval: 15m
```

* `pool_max_entries`: (optional, default is 10000). Set a maximum size of the mempool
* `fragment_ttl` describes for how long the node shall keep a fragment (a _transaction_)
  pending in the pool before being discarded;
* `log_max_entries`: (optional, default is 100000). Set a maximum size of fragment logs
* `garbage_collection_interval` describes the interval between 2 garbage collection
  runs: i.e. when the node removes item (fragments or logs) that have timed out. 
