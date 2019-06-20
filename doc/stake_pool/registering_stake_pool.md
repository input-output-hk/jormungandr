# registering stake pool

There are multiple components to be aware of when running a stake pool:

* your `NodeId`: it is the identifier within the blockchain protocol (wallet
  will delegate to your stake pool via this `NodeId`);
* your [**VRF**] key pairs: this is the cryptographic material we will use to participate
  to the leader election; VRF stands for....
* your [**KES**] key pairs: this is the cryptographic material we will use to sign the
  block with. KES stands for...

So in order to start your stake pool you will need to generate these objects.

# The primitives

## VRF key pair

To generate your [**VRF**] Key pairs, we will utilise [`jcli`] as described
[here](../jcli/key.md):

```
$ jcli key generate --type=Curve25519_2HashDH > stake_pool_vrf.prv
```

`stake_pool_vrf.prv` file now contains the VRF private key.

```
$ cat stake_pool_vrf.prv | jcli key to-public > stake_pool_vrf.pub
```

## KES key pair

Similar to above:

```
$ jcli key generate --type=SumEd25519_12 > stake_pool_kes.prv
```

`stake_pool_kes.prv` now contains your KES private key

```
$ cat stake_pool_kes.prv | jcli key to-public > stake_pool_kes.pub
```

# creating your stake pool certificate

The certificate is what will be sent to the blockchain in order to register
yourself to the other participants of the blockchain that you are a stake
pool too.

```
$ jcli certificate new stake-pool-registration \
    --kes-key $(cat stake_pool_kes.pub) \
    --vrf-key $(cat stake_pool_vrf.pub) \
    --serial 1010101010 > stake_pool.cert
```

Now you need to sign this certificate with the owner key:

```
$ cat stake_pool.cert | jcli certificate sign stake_key.prv | tee stake_pool.cert
cert1qsqqqqqqqqqqqqqqqqqqq0p5avfqp9tzusr26...cegxaz
```

And now you can retrieve your stake pool id (`NodeId`):

```
$ cat stake_pool.cert | jcli certificate get-stake-pool-id | tee stake_pool.id
ea830e5d9647af89a5e9a4d4089e6e855891a533316adf4a42b7bf1372389b74
```

[**VRF**]: https://en.wikipedia.org/wiki/Verifiable_random_function
