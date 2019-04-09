# Starting an Ouroboros-BFT blockchain

BFT stands for the Byzantine Fault Tolerant
([read the paper](https://iohk.io/research/papers/#L5IHCV53)).

Jormungandr allows you to start a BFT blockchain fairly easily. The main
downside is that it is centralized, only a handful of nodes will ever have
the right to create blocks.

## How does it work

It is fairly simple. A given number of Nodes (`N`) will generate
a key pairs of type `Ed25519Extended` (see
[Jormungandr's Keys](./jormungandr_keys.md)).

They all share the public key and add them in the genesis.yaml file.
It is the source of truth, the file that will generate the first block
of the blockchain: the **Block 0**.

Then, ony by one after the other, each Node will be allowed to create a block.
Utilising a Round Robin algorithm.

## Example of genesis file

```yaml
blockchain_configuration:
  block0_date: 1550822014
  discrimination: test
  block0_consensus: bft
initial_setting:
  allow_account_creation: true
  slot_duration: 15
  epoch_stability_depth: 2600
  linear_fees:
    constant: 0
    coefficient: 0
    certificate: 0
  block_version: 1
  bft_leaders:
    - ed25519extended_public1k3wjgdcdcn23k6dwr0cyh88ad7a4ayenyxaherfazwy363pyy8wqppn7j3
    - ed25519extended_public13talprd9grgaqzs42mkm0x2xek5wf9mdf0eefdy8a6dk5grka2gstrp3en
initial_funds:
  - address: ta1svy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvxlswdf0
    value: 10000
```

In order to start your blockchain in BFT mode you need to be sure that:

* `block_version` is set to `1`;
* `bft_leaders` is non empty;

more information regarding the [genesis file here](./genesis_file.md).

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
  signing_key: ed25519extended_secret1vzpkw6lqk5sfaa0rtp64s28s7zcegpwqte0psqneum5w9mcgafd0gwexmfn7s96lqja5sv520zx6hx5hd0qsgahp3ta8grrrxkd8n0cjmaqre
```

Configure your Node (config.yml) and run the following command:

```
$ jormungandr --genesis-block block-0.bin \
    --config example.config \
    --secret node_secret.yaml
```
