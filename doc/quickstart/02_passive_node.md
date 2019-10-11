In order to start the node, you first need to gather the blockchain
information you need to connect to.

1. the hash of the **genesis block** of the blockchain, this will be the source
   of truth of the blockchain. It is 64 hexadecimal characters.
2. the **trusted peers** identifiers and access points.

These information are essentials to start your node in a secure way.

The **genesis block** is the first block of the blockchain. It contains the
static parameters of the blockchain as well as the initial funds. Your node
will utilise the **Hash** to retrieve it from the other peers. It will also
allows the Node to verify the integrity of the downloaded **genesis block**.

The **trusted peers** are the nodes in the public network that your Node will
trust in order to initialise the Peer To Peer network.

# The node configuration

Your node configuration file may look like the following:

**Note**

This config shouldn't work as it is, the ip address and port for the trusted peer should be those of an already running node.
Also, the public_address ('u.x.v.t') should be a valid address (you can use an internal one, eg: 127.0.0.1).
Furthermore, you need to have permission to write in the path specified by the storage config.

```yaml
storage: "/mnt/cardano/storage"

rest:
  listen: "127.0.0.1:8443"

p2p:
  trusted_peers:
    - address: "/ip4/104.24.28.11/tcp/8299"
      id: ed25519_pk14uednfkcqq2u2w4uhy7saahyqapn78huu8fj7tv84rqeer78hcvs8h6r6u
```

Description of the fields:

- `storage`: (optional) Path to the storage. If omitted, the
  blockchain is stored in memory only.
- `log`: (optional) Logging configuration:
    - `level`: log messages minimum severity. If not configured anywhere, defaults to "info".
        Possible values: "off", "critical", "error", "warn", "info", "debug", "trace".
    - `format`: Log output format, `plain` or `json`.
    - `output`: Log output destination. Possible values are:
      - `stdout`: standard output
      - `stderr`: standard error
      - `syslog`: syslog (only available on Unix systems)
      - `journald`: journald service (only available on Linux with systemd,
        (if jormungandr is built with the `systemd` feature)
      - `gelf`: Configuration fields for GELF (Graylog) network logging protocol
        (if jormungandr is built with the `gelf` feature):
        - `backend`: _hostname_:_port_ of a GELF server
        - `log_id`: identifier of the source of the log, for the `host` field
                    in the messages.
- `rest`: (optional) Configuration of the REST endpoint.
    - `listen`: _address_:_port_ to listen for requests
    - `pkcs12`: (optional) Certificate file
    - `cors`: (optional) CORS configuration, if not provided, CORS is disabled
      - `allowed_origins`: (optional) allowed origins, if none provided, echos request origin
      - `max_age_secs`: (optional) maximum CORS caching time in seconds, if none provided, caching is disabled
- `p2p`: P2P network settings
    - `trusted_peers`: (optional) the list of nodes's [multiaddr][multiaddr] with their associated `public_id`
      to connect to in order to bootstrap the P2P topology (and bootstrap our local blockchain);
    - `private_id`: the node's private key ([`Ed25519`]) that will be used to identify this node to the network
    - `public_address`: [multiaddr][multiaddr] string specifying address of the
      P2P service. This is the public address that will be distributed to other
      peers of the network that may find interest in participating to the
      blockchain dissemination with the node.
    - `listen_address`: (optional) [multiaddr][multiaddr] specifies the address the node
      will listen to to receive p2p connection. Can be left empty and the node will listen
      to whatever value was given to `public_address`.
    - `topics_of_interest`: The dissemination topics this node is interested to hear about:
      - `messages`: Transactions and other ledger entries.
        Typical setting for a non-mining node: `low`. For a stakepool: `high`;
      - `blocks`: Notifications about new blocks.
        Typical setting for a non-mining node: `normal`. For a stakepool: `high`.
    - `max_connections`: The maximum number of simultaneous P2P connections
      this node should maintain.
- `explorer`: (optional) Explorer settings
    - `enabled`: True or false

[multiaddr]: https://github.com/multiformats/multiaddr

# Starting the node

```
jormungandr --config config.yaml --genesis-block-hash 'abcdef987654321....'
```

The 'abcdef987654321....' part refers to the hash of the genesis, that should be given to you from one of the peers in the network you are connecting to.

In case you have the genesis file (for example, because you are creating the network) you can get this hash with jcli.

```sh
cat block-0 | jcli genesis hash
```

or, in case you only have the yaml file

```sh
cat genesis.yaml | jcli genesis encode | jcli genesis hash
```

[`Ed25519`]: ../jcli/key.md