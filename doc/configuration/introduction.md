This chapter covers the node documentation, necessary to have a working system. It covers
the network, logging and storage parameters.

The node configuration uses the [YAML](https://en.wikipedia.org/wiki/YAML) format.

This is an example of a configuration file:

```YAML
storage: "/tmp/storage"
log:
  level: debug
  format: json
p2p:
  trusted_peers:
    - address: "/ip4/104.24.28.11/tcp/8299"
      id: 0ccc678e5c41fcffc7398fc5cc9c4e08ba88934fe6565305
    - address: "/ip4/104.24.29.11/tcp/8299"
      id: 328c71454e1ecdf88fc5e3763c74997e117f0dd84ef6eddf
  public_address: "/ip4/127.0.0.1/tcp/8080"
  public_id: ad24537cb009bedaebae3d247fecee9e14c57fe942e9bb0d
  topics_of_interest:
    messages: low
    blocks: normal
```
