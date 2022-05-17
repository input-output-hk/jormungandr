
# Hersir

Hersir is a cli & api project capable of bootstrapping local jormungandr network which can be exercised by various tools.

## build & install

In order to build hersir in main project folder run:
```
cd testing/hersir
cargo build
cargo install --path . --force
```

## quick start

The simplest configuration is available by using command:

`hersir --config res\example.yaml`

it results in small network with all data dumped to current folder

## configuration

Simple example:

```
nodes:
    - spawn_params:
        alias: passive
        leadership_mode: passive
        persistence_mode: inmemory
      trusted_peers:
        - leader
    - spawn_params:
        alias: leader
        leadership_mode: leader
        persistence_mode: inmemory

blockchain:
    discrimination: test
    consensus: bft
```

* nodes sections defines each network node. We can define alias, that is then used to express relations between the nodes and if we keep everything in memory or if node can mint blocks or not.

* blockchain section defines blockchain parameters, like what is the consensus and if we are using test or production addresses discrimination.

### full list of available parameters

#### nodes

* spawn_params
  *  `alias:` string (mandatory) - reference name of the node. Example: "alias",
  *  `bootstrap_from_peers:` bool (optional) - should node bootstrap from trusted peers. By default it is auto-evaluated: If node doesn't have any trusted peers it won't bootstrap from peers,
  *  `faketime:` custom (optional) - inject fake time settings. For example:
      ```
        faketime:  {
            /// Clock drift (1 = no drift, 2 = double speed)
            drift: 1,
            /// Offset from the real clock in seconds
            offset: 2,
        }
      ```
  *  `gossip_interval:` time (optional) - node gossip interval with the rest of the network. Format: `number unit`. For example: `10 s`,
  *  `jormungandr:` path (optional) - path to jormungandr node executable,
  *  `leadership_mode:` enum (optional) - node leadership mode. Possible values:
     * `passive` - node won't be able to produce blocks,
     * `leader` - node will be able to mint blocks,
  *  `listen_address:` string (optional) - override listen address for node. Example: `/ip4/127.0.0.1/tcp/10005`,
  *  `log_level:` enum (optional) - log level, Possible values: (info/warn/error/debug/trace)
  *  `max_bootstrap_attempts:` number (optional) - maximum number of bootstrap attempt before abandon,
  *  `max_connections:` number (optional) - max connection node will create with other nodes,
  *  `max_inbound_connections:` number (optional) - max inbound connection that node will accept,
  *  `mempool:` custom (optional) - mempool configuration. Example:
        ```
        mempool:
            pool_max_entries: 100000
            log_max_entries: 100000
        ```
  *  `network_stuck_check:` time (optional) - check interval which node use to verify blockchain advanced. Format: `number unit`. For example: `10 s`,
  *  `node_key_file:` path (optional) - path to node network key,
  *  `persistence_mode:` enum (optional) - set persistence mode. Possible values:
     * `inmemory` - everything is kept in node memory. If node restarts, all history is gone,
     * `persistence` - node uses local storage to preserve current state,
  *  `persistent_fragment_log:` path (optional) - persistent fragment log serializes every fragment node receives via REST api,
  *  `policy:` custom (optional) - defines nodes quarantine configuration. Example:
        ```
         policy:
            quarantine_duration: 30m
            quarantine_whitelist:
              - "/ip4/13.230.137.72/tcp/3000"
              - "/ip4/13.230.48.191/tcp/3000"
              - "/ip4/18.196.168.220/tcp/3000"
        ```
  *  `preferred_layer:` custom (optional) - defines preferences in gossiping. Example:
        ```
          layers:
            preferred_list:
              view_max: 20
              peers:
                - address: "/ip4/13.230.137.72/tcp/3000"
                  id: e4fda5a674f0838b64cacf6d22bbae38594d7903aba2226f
                - address: "/ip4/13.230.48.191/tcp/3000"
                  id: c32e4e7b9e6541ce124a4bd7a990753df4183ed65ac59e34
                - address: "/ip4/18.196.168.220/tcp/3000"
                  id: 74a9949645cdb06d0358da127e897cbb0a7b92a1d9db8e70
        ```
  *  `public_address:` String (optional)- override public address for node. Example: `/ip4/127.0.0.1/tcp/10005`,
  *  `skip_bootstrap:` bool (optional) - skips node bootstrap step,
  *  `topics_of_interest:` custom (optional) - topics of interests describe how eager node will fetch blocks or transactions:
      ```
      topics_of_interest:
        blocks: normal # Default is normal - set to high for stakepool
        messages: low  # Default is low    - set to high for stakepool
      ```
  *  `verbose:` bool (optional) - enable verbose mode, which prints additional information,

*  `trusted_peers:` List (optional) - list of trusted peers. Example:
    ```
        trusted_peers:
          - leader
          - leader_1
    ```

#### blockchain

* `block0_date:` date (optional) -  block0 date, if not provided current date would be taken,
* `block_content_max_size:` number (optional) - maximum block content size in bytes,
* `committees:` list (optional) - list of wallet aliases which will be committees (capable of tallying the vote),
* `consensus:` enum (optional) - blockchain consensus, possible values: Bft,GenesisPraos,
* `consensus_genesis_praos_active_slot_coeff:` float (optional) - Determines minimum stake required to try becoming slot leader, must be in range (0,1],
* `discrimination:` enum (optional) -  type of discrimination of the blockchain, if this blockchain is meant for production then use `production` otherwise set `test`,
* `external_committees:` list (optional) - list of committees to be included in block0,
* `external_consensus_leader_ids:` list (optional) - list of external leaders id (apart from already defined nodes),
* `external_wallets:` list (optional) - list of external wallets. Example:
```
  external_wallets:
      - alias: Alice
        address: ca1q47vz09320mx2qcs0gspwm47lsm8sh40af305x759vvhm7qyjyluulja80r
        value: 1000000000
        tokens: {}
```
* `kes_update_speed:` number (optional) - the speed to update the KES Key in seconds,
* `linear_fee:` custom (optional) - fee calculations settings,
* `slot_duration:` number (optional) - The slot duration, in seconds, is the time between the creation of 2 blocks,
* `slots_per_epoch:` number (optional) - number of slots in each epoch,
* `tx_max_expiry_epochs:` number (optional) - transaction ttl (expressed in number of epochs).

#### session

* `jormungandr:` path (optional) - override path to jormungandr. By default it's taken from PATH variable,
* `root:` path (optional) - override path to local storage folder. By default all related data is dumped ino TEMP folder,
* `generate_documentation:` bool (optional) - generate documentation files into local storage folder,
* `mode:` enum (optional) - set hersir working mode. By default it's "standard", which just prints information about correct nodes bootstrap. Possible values:
  * monitor - prints current nodes status as progress bar,
  * standard - just prints information about correct nodes bootstrap,
  * interactive - spawn helper cli, which allows to interact with nodes,
* `log:` enum (optional) - log level, Possible values: (info/warn/error/debug/trace),
* `title:` string (optional) - give local storage folder name instead of random one.

### full list of available commands

Full list of commands is available on `hersir --help` command.

```
hersir 0.1.0

USAGE:
    hersir [FLAGS] --config <config>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose

OPTIONS:
    -c, --config <config>
```
