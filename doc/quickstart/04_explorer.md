# Explorer mode

The node can be configured to work as a explorer. This consumes more resources, but makes it possible to query data otherwise not available.

## Configuration

There is two ways of enabling the explorer api. It can either be done by passing the `--enable-explorer` flag on the start arguemnts or by the config file: 

``` yaml
explorer:
    enabled: true
```

#### CORS

For configuring CORS the explorer API, this needs to be done on the REST section of the config, as documented [here](../configuration/network.md).

## API

A graphql interface can be used to query the explorer data, when enabled, two endpoints are available in the [REST interface](03_rest_api.md): `/explorer/graphql` and `/explorer/graphiql` .

The first is the one that queries are made against, for example: 

``` sh
curl \
    -X POST \
    -H "Content-Type: application/json" \
    --data '{'\
        '"query": "{'\
        '   status {'\
        '       latestBlock {'\
        '           chainLength'\
        '           id'\
        '           previousBlock {'\
        '               id'\
        '           }'\
        '       }'\
        '   }'\
        '}"'\
    '}' \
  http://127.0.0.1:8443/explorer/graphql
```

While the second serves an in-browser graphql IDE that can be used to try queries interactively.
