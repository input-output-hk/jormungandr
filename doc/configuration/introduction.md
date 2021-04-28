# Configuration

This chapter covers the node documentation, necessary to have a working system.
It covers the network, logging and storage parameters.

## Node Configuration

This is an common example of a Jörmungandr node configuration file typically named `node-config.yaml`, however your's will vary depending on your needs.
Additionally, this configuration has been tested on a *specific Jörmungandr version* and may change with newer versions.
It's important to keep in mind that the `trusted_peers` portion of this configuration will be different for each Cardano blockchain network.
If you're trying to connect this node to a specific network, you need to know it's genesis block hash, and it's associated list of trusted peers.

Example Configuration - 1:

```YAML
---
log:
  output: stderr
  level:  info
  format: plain

http_fetch_block0_service:
  - https://url/jormungandr-block0/raw/master/data

skip_bootstrap: false # If set to true - will skip the bootstrapping phase

bootstrap_from_trusted_peers: false

p2p:
  public_address: "/ip4/X.X.X.X/tcp/Y" # This should match your public IP address (X) and port number (Y)
  #listen: 0.0.0.0:Y
  topics_of_interest:
    blocks: normal # Default is normal - set to high for stakepool
    messages: low  # Default is low    - set to high for stakepool
  allow_private_addresses: false
  max_connections: 256
  max_client_connections: 192
  max_unreachable_nodes_to_connect_per_event: 20
  gossip_interval: 10s
  max_bootstrap_attempts: # Default is not set
  trusted_peers:
    - address: "/ip4/13.230.137.72/tcp/3000"
      id: e4fda5a674f0838b64cacf6d22bbae38594d7903aba2226f
    - address: "/ip4/13.230.48.191/tcp/3000"
      id: c32e4e7b9e6541ce124a4bd7a990753df4183ed65ac59e34
    - address: "/ip4/18.196.168.220/tcp/3000"
      id: 74a9949645cdb06d0358da127e897cbb0a7b92a1d9db8e70
    - address: "/ip4/3.124.132.123/tcp/3000"
      id: 431214988b71f3da55a342977fea1f3d8cba460d031a839c
    - address: "/ip4/18.184.181.30/tcp/3000"
      id: e9cf7b29019e30d01a658abd32403db85269fe907819949d
    - address: "/ip4/184.169.162.15/tcp/3000"
      id: acaba9c8c4d8ca68ac8bad5fe9bd3a1ae8de13816f40697c
    - address: "/ip4/13.56.87.134/tcp/3000"
      id: bcfc82c9660e28d4dcb4d1c8a390350b18d04496c2ac8474
  policy:
    quarantine_duration: 30m
    quarantine_whitelist:
      - "/ip4/13.230.137.72/tcp/3000"
      - "/ip4/13.230.48.191/tcp/3000"
      - "/ip4/18.196.168.220/tcp/3000"
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

rest:
  listen: 127.0.0.1:3100

storage: "./storage"

explorer:
  enabled: false

mempool:
    pool_max_entries: 100000
    log_max_entries: 100000

leadership:
    logs_capacity: 1024

no_blockchain_updates_warning_interval: 15m

```

Note:
  The node configuration uses the [YAML](https://en.wikipedia.org/wiki/YAML) format.

## Advanced

### Rewards report

Starting the node `jormungandr` with the command line option `--rewards-report-all` will
collect a thorough report of all the reward distribution. It can then be accessed via the
REST endpoints `/api/v0/rewards/history/1` or `/api/v0/rewards/epoch/10`.

**this is not a recommended settings as it may take memory and may trigger some latency**.

### Handling of time-consuming transactions

By default we allow a single transaction to delay a block by 50 slots. This can
be changed by adjusting the `block_hard_deadline` setting.

#### The following is deprecated and will be removed

If you want to record the reward distributions in a directory it is possible to set
the environment variable: `JORMUNGANDR_REWARD_DUMP_DIRECTORY=/PATH/TO/DIR/TO/WRITE/REWARD`.

If an error occur while dumping the reward, the node will **panic** with an appropriate
error message.
