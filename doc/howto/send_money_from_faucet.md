+++
title = "How to send money from faucet?"
author = ["alejandro garcia"]
draft = false
+++

## Create a receiver account {#create-a-receiver-account}

Now we are going to create an account that will receive fonuds from the faucet

```bash
jcli key generate --type=Ed25519Extended > receiver_secret.key
cat receiver_secret.key | jcli key to-public > receiver_public.key
jcli address account --testing $(cat receiver_public.key) | tee receiver_account.txt
```

with the receiver account, we can now send funds.


## Withdraw from faucet {#withdraw-from-faucet}

Now we are going to use the faucet-send-money.sh script that bootstrap created for us:

```bash
chmod +x faucet-send-money.sh
./faucet-send-money.sh $(cat receiver_account.txt) 1000
```

It will show we have a Success!
**but** It's a partial success, what it means is that we have successfully sent the transaction to the blockchain.
Not that the transaction has actually been accepted.
So we need to check the status of the transaction.


## Check that the transaction is in the blockchain {#check-that-the-transaction-is-in-the-blockchain}

```bash
jcli rest v0 message logs -h http://127.0.0.1:8443/api
```

```text
---
- fragment_id: e7665f4fa737048c8f2f056283a4a305c8e422f85f001560c43cde4ef8f25bfc
  last_updated_at: "2019-06-16T02:39:27.247816543Z"
  received_at: "2019-06-16T02:39:27.247816618Z"
  received_from: Rest
  status: Pending
```

If you do it inmeddiately we will see a status of Pending. Wait at least 10 seconds and try again.

```bash
sleep 10
jcli rest v0 message logs -h http://127.0.0.1:8443/api
```

```text
---
- fragment_id: e7665f4fa737048c8f2f056283a4a305c8e422f85f001560c43cde4ef8f25bfc
  last_updated_at: "2019-06-16T02:40:38.027533869Z"
  received_at: "2019-06-16T02:39:27.247816618Z"
  received_from: Rest
  status:
    InABlock: "0.208"
```

Now it shows that it's in a block.


## Review faucet and receivers balance {#review-faucet-and-receivers-balance}


### Check receiver account balance {#check-receiver-account-balance}

Let's check the balance in the faucet account

```bash
jcli rest v0 account get $(cat receiver_account.txt) -h  http://127.0.0.1:8443/api
```

```text
---
counter: 0
delegation: ~
value: 1000
```

We see that we have the 1,000 tokens we sent...


### Check faucet account balance {#check-faucet-account-balance}

```bash
jcli rest v0 account get $FAUCET_ACCOUNT -h  http://127.0.0.1:8443/api
```

```text

---
counter: 1
delegation:
  - 137
  - 100
  - 159
  - 208
  - 207
  - 115
  - 164
  - 132
  - 57
  - 164
  - 112
  - 209
  - 246
  - 212
  - 70
  - 140
  - 237
  - 137
  - 231
  - 121
  - 109
  - 66
  - 226
  - 115
  - 32
  - 13
  - 84
  - 161
  - 74
  - 64
  - 126
  - 254
value: 999998990
```

If you observe we have less tokens than expected. It is because we withdraw 1,000 tokens plus 10 tokens to pay the transaction fee.

And with that we have concluded or basic usage of the self\_node
