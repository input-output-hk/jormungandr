
There's 2 differents network interfaces which are covered by their respective section:

```yaml
rest:
   ...
p2p:
   ...
```

## REST interface configuration

- `listen`: listen address
- `pkcs12`: certificate file (optional)
- `cors`: (optional) CORS configuration, if not provided, CORS is disabled
  - `allowed_origins`: (optional) allowed origins, if none provided, echos request origin
  - `max_age_secs`: (optional) maximum CORS caching time in seconds, if none provided, caching is disabled

## P2P configuration

- `trusted_peers`: (optional) the list of nodes' [multiaddr][multiaddr] to connect to in order to
    bootstrap the p2p topology (and bootstrap our local blockchain);
-  public_address: [multiaddr] The address to listen from and accept connections. This is the external public address that will be distributed to other gossip peers on the network.  Public address port must match listen address port.
-  listen_address:  [multiaddr] Internal address that the node will listen to for 
    receiving p2p connections.  This connection will typically be port forwarded from 
    your router or cable modem. Listen address port must match public address port.
- `topics_of_interest`: the different topics we are interested to hear about:
    - `messages`: notify other peers this node is interested about Transactions
    typical setting for a non mining node: `"low"`. For a stakepool: `"high"`;
    - `blocks`: notify other peers this node is interested about new Blocs.
    typical settings for a non mining node: `"normal"`. For a stakepool: `"high"`.
- `max_connections`: the maximum number of P2P connections this node should
    maintain. If not specified, an internal limit is used by default.

[multiaddr]: https://github.com/multiformats/multiaddr
