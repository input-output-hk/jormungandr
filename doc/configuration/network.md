There are 2 different network interfaces which are covered by their respective section:

```yaml
rest:
   ...
p2p:
   ...
```

## REST interface configuration

- `listen`: listen address
- `tls`: (optional) enables TLS and disables plain HTTP if provided
  - `cert_file`: path to server X.509 certificate chain file, must be PEM-encoded and contain at least 1 item
  - `priv_key_file`: path to server private key file, must be PKCS8 with single PEM-encoded, unencrypted key
- `cors`: (optional) CORS configuration, if not provided, CORS is disabled
  - `allowed_origins`: (optional) allowed origins, if none provided, echos request origin
  - `max_age_secs`: (optional) maximum CORS caching time in seconds, if none provided, caching is disabled

### Configuring TLS

In order to enable TLS there must be provided certificate and private key files.

#### Example generation of files for self-signed TLS

Generate private key

```bash
openssl genrsa -out priv.key 2048
```

Wrap private key in PKCS8

```bash
openssl pkcs8 -topk8 -inform PEM -outform PEM -in priv.key -out priv.pk8 -nocrypt
```

Generate a self-signed certificate for private key

```bash
openssl req -new -key priv.key -out cert_req.csr
openssl x509 -req -days 3650 -in cert_req.csr -signkey priv.key -out cert.crt
```

Use generated files in config

```yaml
rest:
  tls:
    cert_file: cert.crt
    priv_key_file: priv.pk8
```

## P2P configuration

- `trusted_peers`: (optional) the list of nodes' [multiaddr][multiaddr] to connect to in order to
    bootstrap the p2p topology (and bootstrap our local blockchain) with the associated `id` (24 bytes
    in hexadecimal given by the trusted peers to allow initial connection to it).
- `public_address`: [multiaddr][multiaddr] the address to listen from and accept connection
    from. This is the public address that will be distributed to other peers
    of the network that may find interest into participating to the blockchain
    dissemination with the node.  Currently only TCP is supported.
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
  - `blocks`: notify other peers this node is interested about new Blocks.
    typical settings for a non mining node: `"normal"`. For a stakepool: `"high"`.
- `max_connections`: the maximum number of P2P connections this node should
    maintain. If not specified, an internal limit is used by default `[default: 256]`
- `max_client_connections`: the maximum number of client P2P connections this
    node should keep open. `[default: 8]`
- `policy`: (optional) set the setting for the policy module
  - `quarantine_duration` set the time to leave a node in quarantine before allowing
    it back (or not) into the fold.
    It is recommended to leave the default value `[default: 30min]`.
- `max_unreachable_nodes_to_connect_per_event`: (optional) set the maximum number of unreachable nodes
  to contact at a time for every new notification.
  Every time a new propagation event is triggered, the node will select
  randomly a certain amount of unreachable nodes to connect to in addition
  to the one selected by other p2p topology layer `[default: 20]`
- `gossip_interval`: (optional) interval to start gossiping with new nodes,
  changing the value will affect the bandwidth. The more often the node will
  gossip the more bandwidth the node will need. The less often the node gossips
  the less good the resilience to node churn. `[default: 10s]`
- `topology_force_reset_interval`: (optional) If this value is set, it will
  trigger a force reset of the topology layers. The default is to not do
  force the reset. It is recommended to let the protocol handle it.
- `max_bootstrap_attempts`: (optional) number of times to retry bootstrapping from trusted peers.
  If not set, default beavior, the bootstrap process will keep retrying indefinitely, until completed successfully.
  If set to *0* (zero), the node will skip bootstrap all together -- *even if trusted peers are defined*.
  If the node fails to bootstrap from any of the trusted peers and the number of bootstrap retry attempts is exceeded,
  then the node will continue to run without completing the bootstrap process.
  This will allow the node to act as the first node in the p2p network (i.e. genesis node),
  or immediately begin gossip with the trusted peers if any are defined.

### The trusted peers

The trusted peers is a concept that is not fully implemented yet. One of the key element
for now is that this is the first node any node tries to connect in order to meet new nodes.
Right now, as far as we know, only one of them is needed. IOHK provides a few others for
redundancy.

### Setting the `public_id`

This is needed to advertise your node as a trusted peer.
If not set, the node will generate a random ID, which is fine for a regular user.
You can generate a public id with **openssl**, for example: `openssl rand -hex 24`

### `topics_of_interest`

This is optional an optional value to set. The default is:

```yaml
messages: low
blocks: normal
```

These value makes sense for most of the users that are not running stake pools or
that are not even publicly reachable.

However for a publicly reachable node, the recommended setting would be:

```yaml
messages: normal
blocks: normal
```

and for a stake pool

```yaml
messages: high
blocks: high
```

[multiaddr]: https://github.com/multiformats/multiaddr
[`jcli key`]: ../jcli/key.md
