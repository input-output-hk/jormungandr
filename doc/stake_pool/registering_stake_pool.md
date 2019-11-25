# registering stake pool

There are multiple components to be aware of when running a stake pool:

* your `NodeId`: it is the identifier within the blockchain protocol (wallet
  will delegate to your stake pool via this `NodeId`);
* your [**VRF**] key pairs: this is the cryptographic material we will use to participate
  to the leader election;
* your **KES** key pairs: this is the cryptographic material we will use to sign the
  block with.

So in order to start your stake pool you will need to generate these objects.

## The primitives

### VRF key pair

To generate your [**VRF**] Key pairs, we will utilise [`jcli`](../jcli/introduction.md) as described
[here](../jcli/key.md):

```sh
jcli key generate --type=Curve25519_2HashDH > stake_pool_vrf.prv
```

`stake_pool_vrf.prv` file now contains the VRF private key.

```sh
cat stake_pool_vrf.prv | jcli key to-public > stake_pool_vrf.pub
```

### KES key pair

Similar to above:

```sh
jcli key generate --type=SumEd25519_12 > stake_pool_kes.prv
```

`stake_pool_kes.prv` now contains your KES private key

```sh
cat stake_pool_kes.prv | jcli key to-public > stake_pool_kes.pub
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
    --owner $(cat owner_key.pub) \
    --serial 1010101010 > stake_pool.cert
```

The `--operator` flag is optional.

And now you can retrieve your stake pool id (`NodeId`):

```sh
cat stake_pool.cert | jcli certificate get-stake-pool-id | tee stake_pool.id
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
