# Delegating your stake

## how to create the delegation certificate

Stake is concentrated in accounts, and you will need your account public key to
delegate its associated stake.

You will need your:

* account public key: a bech32 string of a public key
* the Stake Pool ID: an hexadecimal string identifying the stake pool you want
  to delegate your stake to.

```sh
jcli certificate new stake-delegation ACCOUNT_PUBLIC_KEY STAKE_POOL_ID > stake_delegation.cert
```

## submitting to a node

The `jcli transaction add-certificate` command should be used to add a certificate **before finalizing** the transaction.

For example:

```sh

...

jcli transaction add-certificate $(cat stake_delegation.cert) --staging tx
jcli transaction finalize CHANGE_ADDRESS --fee-constant 5 --fee-coefficient 2 --fee-certificate 2 --staging tx

...
jcli transaction seal --staging tx
jcli transaction auth --key account_key.prv --staging tx
...

```

The `--fee-certificate` flag indicates the cost of adding a certificate, used for computing the fees, it can be omitted if it is zero.

See [here](../jcli/transaction.md) for more documentation on transaction creation.

## how to sign your delegation certificate

This procedure is needed only for certificates that are to be included
in the `genesis config` file.

We need to make sure that the owner of the account is authorizing this
delegation to happens, and for that we need a cryptographic signature.

We will need the account secret key to create a signature

```sh
cat stake_delegation.cert | jcli certificate sign account_key.prv | tee stake_delegation.signedcert
signedcert1q9uxkxptz3zx7akmugk...7764rq
```

The output can now be added in the `genesis config` file
