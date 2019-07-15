# starting a bft node

BFT stands for the Byzantine Fault Tolerant
([read the paper](https://iohk.io/research/papers/#L5IHCV53)).

Jormungandr allows you to start a BFT blockchain fairly easily. The main
downside is that it is centralized, only a handful of nodes will ever have
the right to create blocks.

## How does it work

It is fairly simple. A given number of Nodes (`N`) will generate
a key pairs of type `Ed25519` (see
[JCLI's Keys](./../jcli/key.md)).

They all share the public key and add them in the genesis.yaml file.
It is the source of truth, the file that will generate the first block
of the blockchain: the **Block 0**.

Then, only by one after the other, each Node will be allowed to create a block.
Utilising a Round Robin algorithm.

## Example of genesis file

```yaml
blockchain_configuration:
  block0_date: 1550822014
  discrimination: test
  block0_consensus: bft
  slots_per_epoch: 5
  slot_duration: 15
  epoch_stability_depth: 10
  consensus_leader_ids:
    - ed25519e_pk1k3wjgdcdcn23k6dwr0cyh88ad7a4ayenyxaherfazwy363pyy8wqppn7j3
    - ed25519e_pk13talprd9grgaqzs42mkm0x2xek5wf9mdf0eefdy8a6dk5grka2gstrp3en
  bft_slots_ratio: 0.220
  consensus_genesis_praos_active_slot_coeff: 0.22
  max_number_of_transactions_per_block: 255
  linear_fees:
    constant: 2
    coefficient: 1
    certificate: 4
  kes_update_speed: 43200
initial:
  - fund:
      - address: ta1svy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvxlswdf0
        value: 10000
  - cert: cert1qgqqqqqqqqqqqqqqqqqqq0p5avfqqmgurpe7s9k7933q0wj420jl5xqvx8lywcu5jcr7fwqa9qmdn93q4nm7c4fsay3mzeqgq3c0slnut9kns08yn2qn80famup7nvgtfuyszqzqrd4lxlt5ylplfu76p8f6ks0ggprzatp2c8rn6ev3hn9dgr38tzful4h0udlwa0536vyrrug7af9ujmrr869afs0yw9gj5x7z24l8sps3zzcmv
  - legacy_fund:
      - address: 48mDfYyQn21iyEPzCfkATEHTwZBcZJqXhRJezmswfvc6Ne89u1axXsiazmgd7SwT8VbafbVnCvyXhBSMhSkPiCezMkqHC4dmxRahRC86SknFu6JF6hwSg8
        value: 123
```

In order to start your blockchain in BFT mode you need to be sure that:

* `consensus_leader_ids` is non empty;

more information regarding the [genesis file here](./01_the_genesis_block.md).

## Creating the block 0

```
jcli genesis encode --input genesis.yaml --output block-0.bin
```

This command will create (or replace) the **Block 0** of the blockchain
from the given genesis configuration file (`genesis.yaml`).

## Starting the node

Now that the blockchain is initialized, you need to start your node.

Write you private key in a file on your HD:

```
$ cat node_secret.yaml
bft:
  signing_key: ed25519_sk1hpvne...
```

Configure your Node (config.yml) and run the following command:

```
$ jormungandr --genesis-block block-0.bin \
    --config example.config \
    --secret node_secret.yaml
```

It's possible to use the flag `--secret` multiple times to run a node
with multiple leaders.

## Step by step to start the BFT node

1. Generate initial config `jcli genesis init > genesis.yaml`
2. Generate secret key, e.g. ` jcli key generate --type=Ed25519 > key.prv`
3. Put secret key in a file, e.g. `node_secret.yaml` as follows:

```yaml
bft:
 signing_key: ed25519_sk1kppercsk06k03yk4qgea....
```

4. Generate public key out of previously generated key ` cat key.prv |  jcli key to-public`
5. Put generated public key as in `genesis.yaml` under `consensus_leader_ids:`
6. Generate block = `jcli genesis encode --input genesis.yaml --output block-0.bin`
7. Create config file and store it on your HD as `node.config` e.g. ->

```yaml
---
log:
  level: trace
  format: json
rest:
  listen: "127.0.0.1:8607"
p2p:
  public_address: /ip4/127.0.0.1/tcp/8606
  topics_of_interest:
    messages: low
    blocks: normal
```

8. Start Jörmungandr node :
```
jormungandr --genesis-block block-0.bin --config node.config --secret node_secret.yaml
```

# Script

Additionally, there is a script [here](https://github.com/input-output-hk/jormungandr/blob/master/scripts/bootstrap) that can be used to bootstrap a test node with bft consensus protocol.
