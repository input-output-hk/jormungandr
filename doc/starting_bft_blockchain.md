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

## Example of genesis configuration file

```yaml
start_time: 1550822014
slot_duration: 15
epoch_stability_depth: 2600
allow_account_creation: true
address_discrimination: Production 
initial_utxos: []
linear_fees:
        constant: 2
        coefficient: 1
        certificate: 4
bft_leaders:
          - ed25519extended_public1k3wjgdcdcn23k6dwr0cyh88ad7a4ayenyxaherfazwy363pyy8wqppn7j3
          - ed25519extended_public13talprd9grgaqzs42mkm0x2xek5wf9mdf0eefdy8a6dk5grka2gstrp3en
```

Or you can generate it with the following command line:

```
jormungandr init \
    --bft-leader=ed25519extended_public1k3wjgdcdcn23k6dwr0cyh88ad7a4ayenyxaherfazwy363pyy8wqppn7j3 \
    --bft-leader=ed25519extended_public13talprd9grgaqzs42mkm0x2xek5wf9mdf0eefdy8a6dk5grka2gstrp3en \
    --discrimination production > genesis.yaml
```

more information regarding the [genesis file here](./genesis_file.md).

## Starting the node

Now that the blockchain is initialized, you need to start your node.

Write you private key in a file on your HD:

```
$ cat private.key
ed25519extended_secret1vzpkw6lqk5sfaa0rtp64s28s7zcegpwqte0psqneum5w9mcgafd0gwexmfn7s96lqja5sv520zx6hx5hd0qsgahp3ta8grrrxkd8n0cjmaqre
```

Configure your Node (config.yml) and run the following command:

```
$ jormungandr start --genesis-config genesis.yaml \
    --config example.config \
    --secret private.key
```
