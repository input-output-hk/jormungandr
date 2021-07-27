# Mempool

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
* `persistent_log`: (optional, disabled by default) log all incoming fragments to log files,
    rotated on a hourly basis. The value is an object, with the `dir` field
    specifying the directory name where log files are stored.

## Persistent logs

A persistent log is a collection of records comprised of a UNIX timestamp of when a fragment was
registereed by the mempool followed by the hex-encoded fragment body. This log is a line-delimited
JSON stream.

Keep in mind that enabling persistent logs could result in impaired performance of the node if disk
operations are slow. Consider using a reasonably fast ssd for best results.
