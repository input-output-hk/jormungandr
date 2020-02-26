# Stake

In a proof of stake, participants are issued a stake equivalent to the amount
of coins they own. The stake is then used to allow participation in the protocol,
simply explained as:

> The more stake one has, the more likely one will participate in the good health of the network.

When using the BFT consensus, the stake doesn't influence how the system
runs, but stake can still be manipulated for a later transition of the chain
to another consensus mode.

## Stake in the Account Model

Account are represented by 1 type of address and are just composed of a public key.
The account accumulate moneys and its stake power is directly represented by the amount it contains

For example:

```

    A - Account with 30$ => Account A has stake of 30
    B - Account with 0$ => Account B has no stake

```

The account might have a bigger stake than what it actually contains, since it could
also have associated UTXOs, and this case is covered in the next section.

## Stake in the UTXO Model

UTXO are represented by two kind of addresses:

* single address: those type of address have no stake associated
* group address: those types of address have an account associated which receive the stake power of the UTXOs value

For example with the following utxos:

```
    UTXO1 60$ (single address) => has stake of 0

    UTXO2 50$ (group address A) \
                                 ->- A - Account with 10$ => Account A has stake of 100
    UTXO3 40$ (group address A) /

    UTXO4 20$ (group address B) -->- B - Account with 5$ => Account B has stake of 25
```

## Stake pool

Stake pool are the trusted block creators in the genesis-praos system. A pool
is declared on the network explicitely by its owners and contains, metadata
and cryptographic material.

Stake pool has no stake power on their own, but participants in the network
delegate their stake to a pool for running the operation.

## Stake Delegation

Stake can and need to be delegated to stake pool in the system. They can
change over time with a publication of a new delegation certificate.

Delegation certificate are a simple declaration statement in the form of:

```
    Account 'A' delegate to Stake Pool 'Z'
```

Effectively it assign the stake in the account and its associated UTXO stake
to the pool it delegates to until another delegation certificate is made.