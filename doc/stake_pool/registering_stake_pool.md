# Registering a stake pool

There are multiple components to be aware of when running a stake pool:

* your `NodeId`: it is the identifier within the blockchain protocol (wallet
  will delegate to your stake pool via this `NodeId`);
* your [**VRF**] key pairs: this is the cryptographic material we will use to participate
  to the leader election;
* your **KES** key pairs: this is the cryptographic material we will use to sign the
  block with.
* the stake pool **Tax**: the value the stake pool will take from the total reward due to
  the stake pool before distributing rewards (if any left) to the delegators.

So in order to start your stake pool you will need to generate these objects.

## The primitives

### VRF key pair

To generate your [**VRF**] Key pairs, we will utilise [`jcli`](../jcli/introduction.md) as described
[here](../jcli/key.md):

```sh
jcli key generate --type=EllipticCurve2hashDhH stake_pool_vrf.prv
```

`stake_pool_vrf.prv` file now contains the VRF private key.

```sh
jcli key to-public --input stake_pool_vrf.prv stake_pool_vrf.pub
```

`stake_pool_vrf.pub` file now contains the VRF public key.

### KES key pair

Similar to above:

```sh
jcli key generate --type=SumEd25519_12 stake_pool_kes.prv
```

`stake_pool_kes.prv` file now contains the KES private key

```sh
jcli key to-public --input stake_pool_kes.prv stake_pool_kes.pub
```

`stake_pool_kes.pub` file now contains the KES public key

## Choosing the **Tax** parameters

There are 3 values you can set to configure the stake pool's **Tax**:

* `tax-fixed`: this is the fixed cut the stake pool will take from the total reward due to
  the stake pool;
* `tax-ratio`: this is the percentage of the remaining value that will be taken from the total due
* `tax-limit`: a value that can be set to limit the pool's **Tax**.

All of these values are optionals, if not set, they will be set to `0`. This will mean
no tax for the stake pool: rewards are all distributed to the delegators.

### So how does this works

Let say you control a stake pool `SP`, with 2 owners (`O1` and `O2`). During epoch 1, `SP` has
created some blocks and is entitled to receive `10_000`.

Before distributing the `10_000` among the delegators, `SP` will take its **Tax**.

1. we extract the `tax-fixed`. If this is greater or equal to the total (`10_000`)
   then we stop there, there is no more rewards to distribute.
2. with what remains the `SP` extracts its `tax-ratio` and checks the **tax** from the ratio
   is not greater than `tax-limit`.
3. the total `SP` rewards will then be distributed equally to the owners (O1 and O2).
   Note that if the `--reward-account` is set, the rewards for `SP` are then distributed
   to that account and nothing to `O1` and `O2`.

For example:

|                       | total | fixed | ratio | limit | `SP`  | `O1`  | `O2`  | for delegators |
| --------------------- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :------------: |
| takes 100%            | 10000 |   0   |  1/1  |   0   | 10000 | 5000  | 5000  |       0        |
| fixed of 1000         | 10000 | 1000  |  0/1  |   0   | 1000  |  500  |  500  |      9000      |
| fixed + 10%           | 2000  | 1000  | 1/10  |   0   | 1100  |  550  |  550  |      900       |
| fixed + 20% up to 150 | 2000  | 1000  |  1/5  |  150  | 1150  |  575  |  575  |      850       |

### The options to set

```
--tax-limit <TAX_LIMIT>
    The maximum tax value the stake pool will take.

    This will set the maximum the stake pool value will reserve for themselves from the `--tax-ratio` (excluding `--tax-fixed`).
--tax-ratio <TAX_RATIO>
    The percentage take of the stake pool.

    Once the `tax-fixed` has been take, this is the percentage the stake pool will take for themselves. [default: 0/1]
--tax-fixed <TAX_VALUE>
    set the fixed value tax the stake pool will reserve from the reward

    For example, a stake pool may set this value to cover their fixed operation costs. [default: 0]
```

## creating a stake pool certificate

The certificate is what will be sent to the blockchain in order to register
yourself to the other participants of the blockchain that you are a stake
pool too.

```sh
jcli certificate new stake-pool-registration \
    --kes-key $(cat stake_pool_kes.pub) \
    --vrf-key $(cat stake_pool_vrf.pub) \
    --start-validity 0 \
    --management-threshold 1 \
    --tax-fixed 1000000 \
    --tax-limit 1000000000 \
    --tax-ratio "1/10" \
    --owner $(cat owner_key.pub) > stake_pool.cert
```

The `--operator` flag is optional.

And now you can retrieve your stake pool id (`NodeId`):

```sh
jcli certificate get-stake-pool-id stake_pool.cert
ea830e5d9647af89a5e9a4d4089e6e855891a533316adf4a42b7bf1372389b74
```

[**VRF**]: https://en.wikipedia.org/wiki/Verifiable_random_function

## submitting to a node

The `jcli transaction add-certificate` command should be used to add a certificate **before finalizing** the transaction.

For example:

```sh
...

jcli transaction add-certificate $(cat stake_pool.cert) --staging tx
jcli transaction finalize CHANGE_ADDRESS --fee-constant 5 --fee-coefficient 2 --fee-certificate 2 --staging tx

...
jcli transaction seal --staging tx
jcli transaction auth --key owner_key.prv --staging tx
...
```

The `--fee-certificate` flag indicates the cost of adding a certificate, used for computing the fees, it can be omitted if it is zero.

See [here](../jcli/transaction.md) for more documentation on transaction creation.
