When running an active node (BFT leader or stake pool) it is interesting to be
able to make choices on how to manage the pending transactions: how long to keep
them, how to prioritize them etc.

The `mempool` field in your node config file is not mandatory, by default it is set
as follow:

```yaml
mempool:
    fragment_ttl: 30m
    log_ttl: 1h
    garbage_collection_interval: 15m
```

* `fragment_ttl` describes for how long the node shall keep a fragment (a _transaction_)
  pending in the pool before being discarded;
* `log_ttl` describes for how long the node will keep logs of pending/accepted/rejected
  fragments in the pool; This is link to the data you receives from the REST fragment
  logs end point;
* `garbage_collection_interval` describes the interval between 2 garbage collection
  runs: i.e. when the node removes item (fragments or logs) that have timed out. 
