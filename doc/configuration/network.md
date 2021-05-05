# Node network

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
  - `allowed_origins`: (optional) allowed origins, if none provided, echos request origin, note that
    an origin should include a scheme, for example: `http://127.0.0.1:8080`.
  - `max_age_secs`: (optional) maximum CORS caching time in seconds, if none provided, caching is disabled

### Configuring TLS

In order to enable TLS there must be provided certificate and private key files.

#### `jcli` TLS requirements.

Note that `jormungandr` itself does not have any specific requirements for TLS certificates and you
may give whatever you want including self-signed certificates as long as you do not intend to use
`jcli`.

The cryptography standards used by `jcli` as well as by all modern browsers and many http clients
place the following requirements on certificates:

- A certificate should adhere to X.509 v3 with appropriate key usage settings and subject
  alternative name.
- A certificate must not be self-signed.

Given that, your options are to either get a certificate from a well-known CA (Let's Encrypt will
do, `jcli` uses Mozilla's CA bundle for verification) or create your own local CA and provide the
root certificate to `jcli` via the `--tls-cert-path` option.

#### Creating a local CA using OpenSSL and EasyRSA

EasyRSA is a set of scripts that use OpenSSL and give you an easier experience with setting up your
local CA. You can download them [here](https://github.com/OpenVPN/easy-rsa).

1. Go to `easy-rsa/easy-rsa3`.
2. Configure your CA. To do that, create the configuration file (`cp vars.example vars`); open it
   with the text editor of your choise (for example, `vim vars`); uncomment and edit fields you
   need to change. Each CA needs to edit these lines (find then in your `vars` file according to
   their organization structure:

    #set_var.EASYRSA_REQ_COUNTRY----"US"
    #set_var.EASYRSA_REQ_PROVINCE---"California"
    #set_var.EASYRSA_REQ_CITY---"San.Francisco"
    #set_var.EASYRSA_REQ_ORG----"Copyleft.Certificate.Co"
    #set_var.EASYRSA_REQ_EMAIL--"me@example.net"
    #set_var.EASYRSA_REQ_OU-----"My.Organizational.Unit"

3. When your configuration is ready, run `./easyrsa init-pki` and `./easyrsa build-ca nopass`. You
   will be prompted to set the name of your CA.
4. Run `./easyrsa gen-req server nopass` to create a new private key and a certificate signing
   request. You will be prompted to enter the host name (`localhost` for local testing).
5. Run `./easyrsa sign-req server server` to sign the request.

To use the generated certificate, use it and the corresponding key in your `jormungandr` config:

```yaml
rest:
  tls:
    cert_file: <path to server.crt>
    priv_key_file: <path to server.key>
```

Use the CA certificate with `jcli`.

## P2P configuration

- `trusted_peers`: (optional) the list of nodes' [multiaddr][multiaddr] to connect to in order to
    bootstrap the p2p topology (and bootstrap our local blockchain). Note that you can use a DNS
    name in the following format: `/dns4/node.example.com/tcp/3000`. Use `dns6` instead of `dns4`
    if you want the peer to connect with IPv6.
- `public_address`: [multiaddr][multiaddr] the address to listen from and accept connection
    from. This is the public address that will be distributed to other peers
    of the network that may find interest into participating to the blockchain
    dissemination with the node.  Currently only TCP is supported.
- `node_key_file`: (optional) Path to a file containing a bech32-encoded ed25519 secret key.
  The keys are used to advertize the node in network gossip and to authenticate
  a connection to the node if the node is used as a trusted peer.
  **Most of the users don't need to set this value** as the key will be randomly
  generated if the option is not present.
- `listen`: (optional) socket address (IP address and port separated by a comma),
    specifies the interface address and port the node
    will listen at to receive p2p connection. Can be left empty and the node will listen
    to whatever value was given to `public_address`.
- `topics_of_interest`: (optional) the different topics we are interested to hear about:
  - `messages`: notify other peers this node is interested about Transactions
    typical setting for a non mining node: `"low"`. For a stakepool: `"high"`;
  - `blocks`: notify other peers this node is interested about new Blocks.
    typical settings for a non mining node: `"normal"`. For a stakepool: `"high"`.
- `max_connections`: the maximum number of P2P connections this node should
    maintain. If not specified, an internal limit is used by default `[default: 256]`
- `max_inbound_connections`: the maximum number of client P2P connections this
    node should keep open. `[default: 192]`
- `policy`: (optional) set the setting for the policy module
  - `quarantine_duration` set the time to leave a node in quarantine before allowing
    it back (or not) into the fold.
    It is recommended to leave the default value `[default: 30min]`.
  - `quarantine_whitelist` set a trusted list of peers that will not be quarantined in any circumstance.
    It should be a list of valid addresses, for example: `["/ip4/127.0.0.1/tcp/3000"]`.
    By default this list is empty, `[default: []]`.
- `layers`: (optional) set the settings for some of the poldercast custom layers (see below)
- `max_unreachable_nodes_to_connect_per_event`: (optional) set the maximum number of unreachable nodes
  to contact at a time for every new notification.
  Every time a new propagation event is triggered, the node will select
  randomly a certain amount of unreachable nodes to connect to in addition
  to the one selected by other p2p topology layer `[default: 20]`
- `gossip_interval`: (optional) interval to start gossiping with new nodes,
  changing the value will affect the bandwidth. The more often the node will
  gossip the more bandwidth the node will need. The less often the node gossips
  the less good the resilience to node churn. `[default: 10s]`
- `max_bootstrap_attempts`: (optional) number of times to retry bootstrapping from trusted peers.
  If not set, default behavior, the bootstrap process will keep retrying indefinitely, until completed successfully.
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

### Layers

Jörmungandr provides multiple additional layers to the `poldercast` default ones:
the preferred list or the bottle in the sea.

#### Preferred list

this is a special list that allows to connect multiple nodes together without relying
on the auto peer discovery. All entries in the preferred list are also whitelisted
automatically, so they cannot be quarantined.

##### configuration:

- `view_max`: this is the number of entries to show in the view each round
  the layer will **randomly** select up to `view_max` entries from the whole
  preferred_list.peers list of entries. [default: 20]
- `peers`: the list of peers to keep in the preferred list [default: EMPTY]

Also, the preferred list will never be quarantined or blacklisted, the node will
attempt to connect to (up to `view_max` of) these nodes every time, even if some
are down, unreachable or not operated anymore.

**COMPATIBILITY NOTE**: in near future the peer list will be only a list of addresses and the **ID**
part will not be necessary.

##### Example:

```yaml
p2p:
  layers:
    preferred_list:
      view_max: 20
      peers:
        - address: '/ip4/127.0.0.1/tcp/2029'
          id: 019abc...
        - ...
```

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
