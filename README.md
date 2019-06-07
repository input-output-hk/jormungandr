# Full Node

> Just because you call something a blockchain, that doesn't mean you aren't subject to normal engineering laws.

Documentation available [here](https://input-output-hk.github.io/jormungandr)

## How to install from sources

Currently the minimum supported version of the rust compiler is 1.35, however we recommend to utilise the most recent stable version of the rust compiler.

1. [install rustup](https://www.rust-lang.org/tools/install)
2. run `rustup install stable`
3. run `rustup default stable`
4. clone this repository: `git clone https://github.com/input-output-hk/jormungandr`
5. make sure you have cloned the submodule too: `git submodule update --init --recursive`
6. install **jormungandr**: `cargo install --path jormungandr`
6. install **jcli**: `cargo install --path jcli`

Note:

* on windows, you'll need to add the `/userProfile/.cargo/bin` into the Path;
* on linux and OSX: add `${HOME}/.cargo/bin` to your `${PATH}`
* on linux with systemd: to enable logging to journald replace step 5. with `cargo install --path . --features systemd`

This will install 2 tools:

* `jormungandr`: the cardano node;
* `jcli`: a command line tool to help you use and setup the cardano node;

## How To Use

In order to use jormungandr you need to configure your blockchain and
configure your node.

* the Genesis File is the source of truth, the configuration of the blockchain;
* the Node Configuration file is the configuration of the node (logging, peer addresses...);

### Node Configuration

Example of node config:

```YAML
storage: "/tmp/storage"
logger:
  verbosity: 1
  format: json
peer_2_peer:
  trusted_peers:
    - id: 1
      address: "/ip4/104.24.28.11/tcp/8299"
    - id: 2
      address: "/ip4/104.24.29.11/tcp/8299"
  public_address: "/ip4/127.0.0.1/tcp/8080"
  topics_of_interests:
    messages: low
    blocks: normal
```

Fields description:

  - *storage*: (optional) path to the storage. If omitted, the
    blockchain is stored in memory only.
  - *logger*: (optional) logger configuration,
    - *verbosity*: 
      - 0: warning
      - 1: info
      - 2: debug
      - 3 and above: trace
    - *format*: log output format - plain or json.
    - *output*: log output - stderr, syslog (unix only) or journald (linux with systemd only, must be enabled during compilation)
  - *rest*: (optional) configuration of the rest endpoint.
    - *listen*: listen address
    - *pkcs12*: certificate file (optional)
    - *prefix*: (optional) api prefix
  - *peer_2_peer*: the P2P network settings
    - *trusted_peers*: (optional) the list of nodes to connect to in order to
      bootstrap the p2p topology (and bootstrap our local blockchain);
    - *public_id*: (optional) the public identifier send to the other nodes in the
      p2p network. If not set it will be randomly generated.
    - *public_access*: the address to listen from and accept connection
      from. This is the public address that will be distributed to other peers
      of the network that may find interest into participating to the blockchain
      dissemination with the node;
    - *topics_of_interests*: the different topics we are interested to hear about:
      - *messages*: notify other peers this node is interested about Transactions
        typical setting for a non mining node: `"low"`. For a stakepool: `"high"`;
      - *blocks*: notify other peers this node is interested about new Blocs.
        typical settings for a non mining node: `"normal"`. For a stakepool: `"high"`;

### Starting the node

If you are not a leader node, then you can start the jormundandr with:

```sh
jormungandr --genesis-block block-0.bin \
  --config example.config
```

# documentations

Documentation is available as markdown [here](doc/SUMMARY.md)

* [internal design](./doc/internal_design.md) of jormungandr

# Extra tooling

## CLI

Building:

```sh
cargo build --bin jcli
```

# License

This project is licensed under either of the following licenses:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

Please choose the licence you want to use.
