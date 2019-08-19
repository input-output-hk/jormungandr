The `leadership` field in your node config file is not mandatory, by default it is set
as follow:

```yaml
leadership:
    log_ttl: 1h
    garbage_collection_interval: 15m
```

* `log_ttl` describes for how long the node will keep logs of leader events.
  This is link to the data you receives from the REST leadership logs end point;
* `garbage_collection_interval` describes the interval between 2 garbage collection
  runs: i.e. when the node removes item logs that have timed out
