+++
title = "How to send money from the faucet to an account address?"
author = ["alejandro garcia"]
draft = false
+++

Follow the instructions below or watch this video tutorial: [Jormungandr send transactions](https://youtu.be/6YFoitp-hsw)

On this tutorial we are going to go beyond simply setting up a node to actually transfer tokens from one account (the faucet) to another.
The faucet was created by the `bootstrap` script, and configured in the `genesis.yaml` file.
Now we are going to create a different account by ourselves and transfer funds to it.

It's important to note that there are two ways to do this by: account address and by UTXO.
In this tutorial we cover the first one.


## Creating a receiver account {#creating-a-receiver-account}

Now we are going to create an account that will receive funds from the faucet

```bash
jcli key generae --type=Ed25519Extended > receiver_secret.key
cat receiver_secret.key | jcli key to-public > receiver_public.key
jcli address account --testing $(cat receiver_public.key) | tee receiver_account.txt
```

```text
ta1shl6qd56d2ngld50tewfjh52hnv6ktkdpqe2k3achj36mcwdcjj5k7al25c
```

with the receiver account, we can now send funds.


## Withdrawing from the faucet {#withdrawing-from-the-faucet}

Now we are going to use the faucet-send-money.sh script that the bootstrap script created for us:

```bash
chmod +x faucet-send-money.sh
./faucet-send-money.sh $(cat receiver_account.txt) 1000
```

```text
## Sending 1000 to ta1shl6qd56d2ngld50tewfjh52hnv6ktkdpqe2k3achj36mcwdcjj5k7al25c
discrimination: testing
account: ed25519_pk1l7srdxn2568mdr67tjv4az4umx4janggx2450w9u5wk7rnwy549s65kjw8
Success!
```

It will show a Success! message **but** this is a partial success. It means that the transaction was successfully created and submitted to the node. Next step is for the node to check the transaction and to include it (or not) to the blockchain. So next we need to wait for a new block to be created in order for the transaction to take effect.
Keep in mind, that blocks are created differently depending on the selected consensus mode (BFT or Genesis).


## Checking that the transaction is in the blockchain {#checking-that-the-transaction-is-in-the-blockchain}

```bash
jcli rest v0 message logs -h http://127.0.0.1:8443/api
```

```text
---
- fragment_id: 4526bc7017e8600f0916ebdf2ac296be9925327c8cc12d3ba91fd1e7b33cb6b2
  last_updated_at: "2019-06-19T01:00:12.053811951+00:00"
  received_at: "2019-06-19T01:00:12.053811836+00:00"
  received_from: Rest
  status: Pending
```

If you do it immediately you will see a status of Pending. Wait and try again until the transaction is **InABlock**.
The waiting (slot) time is variable in the Genesis consensus and fixed in a BFT consensus.

```bash
sleep 20
jcli rest v0 message logs -h http://127.0.0.1:8443/api
```

```text
---
- fragment_id: 4526bc7017e8600f0916ebdf2ac296be9925327c8cc12d3ba91fd1e7b33cb6b2
  last_updated_at: "2019-06-19T01:02:14.013149863+00:00"
  received_at: "2019-06-19T01:00:12.053811836+00:00"
  received_from: Rest
  status:
    InABlock:
      date: "0.49"
```

Now the transaction was accepted by the node and included into block 49.


## Reviewing the faucet and receiver balances {#reviewing-the-faucet-and-receiver-balances}


### Checking the receiver account balance {#checking-the-receiver-account-balance}

Let's check the balance of the faucet account

<a id="code-snippet--receiver-account-balance"></a>
```bash
jcli rest v0 account get $(cat receiver_account.txt) -h  http://127.0.0.1:8443/api
```

```text
---
counter: 0
delegation: ~
value: 1000
```

We see that we have the 1,000 tokens we sent.


### Checking the faucet account balance {#checking-the-faucet-account-balance}

```bash
jcli rest v0 account get $FAUCET_ACCOUNT -h  http://127.0.0.1:8443/api
```

```text
---
counter: 1
delegation:
  - 36
  - 135
  - 250
  - 129
  - 133
  - 190
  - 3
  - 151
  - 18
  - 98
  - 152
  - 190
  - 118
  - 34
  - 196
  - 68
  - 84
  - 248
  - 116
  - 122
  - 168
  - 187
  - 130
  - 249
  - 72
  - 39
  - 25
  - 27
  - 67
  - 126
  - 213
  - 179
value: 999998990
```

Notice how the transaction `counter` has incremented and that there are less tokens than expected. It is because we withdraw 1,000 tokens plus 10 tokens to pay the transaction fee.

The transaction fee, was configured inside the `genesis.yaml` file in `linear_fees`.

The above steps concludes the basic usage of the self node.
