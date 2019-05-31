# starting a genesis blockchain

When starting a genesis praos blockchain there is an element to take
into consideration while constructing the block 0: _the stake distribution_.

In the context of Genesis/Praos the network is fully decentralized and it is
necessary to think ahead about initial stake pools and to make sure there
is stake delegated to these stake pools.

In your genesis yaml file, make sure to set the following values to the appropriate
values/desired values:

```yaml
# The Blockchain Configuration defines the settings of the blockchain.
blockchain_configuration:
  block0_consensus: genesis
  bft_slots_ratio: 0
  consensus_genesis_praos_active_slot_coeff: 0.1
  kes_update_speed: 43200 # 12hours
```

`block0_consensus` set to `genesis` means you want to start a blockchain with
genesis praos as the consensus layer.

`bft_slots_ratio` needs to be set to `0` (we don't support composite modes between
BFT mode and Genesis mode -- yet).

`consensus_genesis_praos_active_slot_coeff` determines minimum stake required to
try becoming slot leader, must be in range 0 exclusive and 1 inclusive.

## The initial certificates

In the `initial_certs` field you will set the initial certificate. This is important
to declare the stake pool and delegate stake to them. Otherwise no block will be ever
created.

Remember that in this array the **order** matters:

1. in order to create a stake pool, you need a stake key, so it needs to be registered
   first;
2. in order to delegate your stake, you need a stake pool to already exists **and** you
   stake key declaration to be set

### The stake key declaration

To create a stake key registration certificate: simply follow the
[registering stake key guide](../stake_pool/registering_stake.md).

### Stake pool registration

Now that you have a stake owner declared in the block0 you can register a stake pool.
Follow the instruction in [registering stake pool guide](../stake_pool/registering_stake_pool.md).

The _owner key_ (the key you sign the stake pool registration certificate) is the secret
key associated to a previously registered stake key.

### Delegating stake

Now that there is both your stake key registered and there are stake pools available
in the block0 you need to delegate to one of the stake pool. Follow the instruction
in [delegating stake](../stake_pool/delegating_stake.md).

And in the initial funds start adding the addresses. To create an address with delegation
follow the instruction in [JCLI's address guide](../jcli/address.md). Utilise the stake key
registered previously as group address:

```
jcli address single $(wallet_key.pub) $(stake_key.pub)
ta1sjx4j3jwel94g0cgwzq9au7h6m8f5q3qnyh0gfnryl3xan6qnmjse3k2uv062mzj34eacjnxthxqv8fvdcn6f4xhxwa7ms729ak3gsl4qrq2mm
```

You will notice that addresses with delegation are longer (about twice longer) than
address without delegation.

For example, the most minimal setting you may have is:

```yaml
initial_certs:
  # register a stake key (K)
  - cert1q8rv4ccl54k99rtnm39xvhwvqcwjcm385n2dwvamahpu5tmdz3pl2qgqgp6lh9x0mngzy5krzw6fgkhkcvkjj3e64qveny82fgzlyfqf62hsfdup8us3h4rayn66wlt97u6e4syu07grm9sghxy3zdjm0quu8eqrdfpysq

  # register a stake pool (P), owner of the stake pool is the stake key (K)
  - cert1qsqqqqqqqqqqqqqqqqqqq0p5avfqp9tzusr26chayeddkkmdlap6tl23ceca8unsghc22tap8clhrzslkehdycufa4ywvqvs4u36zctw4ydtg7xagprfgz0vuujh3lgtxgfszqzqj4xk4sxxyg392p5nqz8s7ev5wna7eqz7ycsuas05mrupmdsfk0fqqudanew6c0nckf5tsp0lgnk8e8j0dpnxvjk2usn52vs8umr3qrccegxaz

  # delegate stake associated to stake key (K) to stake pool (P)
  - cert1q0rv4ccl54k99rtnm39xvhwvqcwjcm385n2dwvamahpu5tmdz3plt65rpewev3a03xj7nfx5pz0xap2cjxjnxvt2ma9y9dalzder3xm5qyqyq0lx05ggrws0ghuffqrg7scqzdsd665v4m7087eam5zvw4f26v2tsea3ujrxly243sgqkn42uttk5juvq78ajvfx9ttcmj05lfuwtq9qhdxzr0

initial_funds:
  # address without delegation
  - address: ta1swx4j3jwel94g0cgwzq9au7h6m8f5q3qnyh0gfnryl3xan6qnmjsczt057x
    value: 10000
  # address delegating to stake key (K)
  - address: ta1sjx4j3jwel94g0cgwzq9au7h6m8f5q3qnyh0gfnryl3xan6qnmjse3k2uv062mzj34eacjnxthxqv8fvdcn6f4xhxwa7ms729ak3gsl4qrq2mm
    value: 1000000
```
