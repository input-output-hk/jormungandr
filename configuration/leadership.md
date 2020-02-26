The `leadership` field in your node config file is not mandatory, by default it is set
as follow:

```yaml
leadership:
    logs_capacity: 1024
```

* `logs_capacity`: the maximum number of logs to keep in memory. Once the capacity
  is reached, older logs will be removed in order to leave more space for new ones
  [default: 1024]
