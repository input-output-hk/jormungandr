It is possible to query the node via its REST Interface.

In the node configuration, you have set something like:

```yaml
# ...

rest:
  listen: "127.0.0.1:8443"
  prefix: "api"

#...
```

This is the REST endpoint to talk to the node, to query blocks or send transaction.

It is possible to query the node stats with the following end point:

```
curl http://127.0.0.1:8443/api/v0/node/stats
```

The result may be:

```json
{"blockRecvCnt":120,"txRecvCnt":92,"uptime":245}
```

> THE REST API IS STILL UNDER DEVELOPMENT

Please note that the end points and the results may change in the future.
