
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
    bootstrap the p2p topology (and bootstrap our local blockchain) with the associated `id` (24 bytes
    in hexadecimal given by the trusted peers to allow initial connection to it).
- `public_address`: [multiaddr][multiaddr] the address to listen from and accept connection
    from. This is the public address that will be distributed to other peers
    of the network that may find interest into participating to the blockchain
    dissemination with the node;
- `public_id`: (optional) This is a static identifier, 24 bytes encoded in hexadecimal. They are used
  to bootstrap the connection to the node if the node introduce itself as a trusted peer.
  **Most of the user don't need to set this value** and in fact we are working toward potentially
  removing the need for this value.
- `listen_address`: (optional) [multiaddr][multiaddr] specifies the address the node
    will listen to to receive p2p connection. Can be left empty and the node will listen
    to whatever value was given to `public_address`.
- `topics_of_interest`: (optional) the different topics we are interested to hear about:
    - `messages`: notify other peers this node is interested about Transactions
    typical setting for a non mining node: `"low"`. For a stakepool: `"high"`;
    - `blocks`: notify other peers this node is interested about new Blocs.
    typical settings for a non mining node: `"normal"`. For a stakepool: `"high"`.
- `max_connections`: the maximum number of P2P connections this node should
    maintain. If not specified, an internal limit is used by default.

### The trusted peers

The trusted peers is a concept that is not fully implemented yet. One of the key element
for now is that this is the first node any node tries to connect in order to meet new nodes.
Right now, as far as we know, only one of them is needed. IOHK provides a few others for
redundancy.

### Setting the `public_id`

Unless you want to advertise your node as a trusted peer, you don't want to set a `public_id`.
This is completely useful. If not set, the node will generate a random one automatically.

### `topics_of_interest`

This is optional an optional value to set. The default is:

```
messages: low
blocks: normal
```

These value makes sense for most of the users that are not running stake pools or
that are not even publicly reachable.

However for a publicly reachable node, the recommended setting would be:

```
messages: normal
blocks: normal
```

and for a stake pool

```
messages: high
blocks: high
```

[multiaddr]: https://github.com/multiformats/multiaddr
[`jcli key`]: ../jcli/key.md