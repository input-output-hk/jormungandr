+++
title = "How to delegate from a new account?"
author = ["alejandro garcia"]
draft = false
+++

Follow the instructions below or watch this video tutorial: [Jormungandr Delegating stake script](https://youtu.be/cr4vTPPE8ps)

In this case we are going to use the `create-account-and-delegate.sh` script

The script is quite complex, so in this section we describe what the scripts does.


## Creating  new account {#creating-new-account}

The start of this script creates a new account with an address.

```text
##
# 1. create an account
##


# send money to this address

## Sending 1000 to ta1shap40ruglnter32nv6vj5084ew8rrxplkv52zakak2jpl8yhmdmss8raqw
discrimination: testing
account: ed25519_pk1lgdtclz8u67gu25mxny4reawt3cces0an9zshdhdj5s0ee97mwuq46qccv
Success!

```

Then we need to check the status of the block, as we have done before:

```text
jcli rest v0 message logs -h http://127.0.0.1:8443/api
```

And the blockchain shows that creating the new account is pending:

```text
---
- fragment_id: cd78d7490be3991749bcc1132d8eaa62c51e604f9e2b2185d9a3d74793ea46fa
  last_updated_at: "2019-06-19T03:45:52.893665379+00:00"
  received_at: "2019-06-19T03:45:52.893665301+00:00"
  received_from: Rest
  status: Pending
---
```


### Verifying the account information {#verifying-the-account-information}

Once the transaction to create the new account is completed we can retrieve the account information.

```text
jcli rest v0 account get ta1shap40ruglnter32nv6vj5084ew8rrxplkv52zakak2jpl8yhmdmss8raqw -h "http://127.0.0.1:8443/api"
```

And we can see that the transaction counter is 0,  the stake is **not** delegated and the balance is 1,000 tokens:

```text
---
counter: 0
delegation: ~
value: 1000
```


## Creating a new delegation certificate {#creating-a-new-delegation-certificate}

The script continues to the next step of creating a delegate certificate.

```text
##
# 2. create a new certificate to delegate our new address's stake
#    to a stake pool
##

creating certificate
```

Once the certificate is created we need to submit it to the blockchain. This is done as any other transaction.

```text
##
# 3. now create a transaction and sign it
##

Success!

```

We know that, the `Success!` message means that it the transaction has been sent but **not** that it was accepted.


### Monitoring the logs {#monitoring-the-logs}

So now we need to monitor the logs:

```text
jcli rest v0 message logs -h http://127.0.0.1:8443/api
```

```text
---
- fragment_id: cd78d7490be3991749bcc1132d8eaa62c51e604f9e2b2185d9a3d74793ea46fa
  last_updated_at: "2019-06-19T03:49:14.013565426+00:00"
  received_at: "2019-06-19T03:45:52.893665301+00:00"
  received_from: Rest
  status:
    InABlock:
      date: "0.1051"
- fragment_id: 694eb37213e14f035a940a4fc6c72ff6adf0df4237d2b88806803a3738da91a4
  last_updated_at: "2019-06-19T03:51:52.997763822+00:00"
  received_at: "2019-06-19T03:51:52.997763774+00:00"
  received_from: Rest
  status: Pending
```

We see that the second transaction, where we delegate the stake is **Pending**. This is valid once the transaction is `InABlock`.


### Checking the delegation and balance in the account {#checking-the-delegation-and-balance-in-the-account}

We can check the balance in the account:

```text
jcli rest v0 account get ta1shap40ruglnter32nv6vj5084ew8rrxplkv52zakak2jpl8yhmdmss8raqw -h "http://127.0.0.1:8443/api"
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
 value: 990
```

In the results you can observe:

-   Transaction counter is increased to 1
-   The delegation Pool ID.
-   The balance (value) has decreased in 10 tokens. This is due to the fact that we paid the fee to send the certificate.

With this we end our quick summary on how to delegate stake.
