# Delegating your stake


## how to create the delegation certificate

Stake is concentrated in accounts, and you will need your account key to
delegate its associated stake.

You will need your:

* account public key: a bech32 string of a public key.
* the Stake Pool ID: an hexadecimal string identifying the stake pool to which you want
  to delegate your stake.

```
$ jcli certificate new stake-delegation STAKE_POOL_ID ACCOUNT_PUBLIC_KEY > stake_delegation.cert
```

## how to sign your delegation certificate

We need to make sure that the owner of the account is authorizing this
delegation to happen, and for that we need a cryptographic signature.

We will need the account secret key to create a signature

```
$ cat stake_delegation.cert | jcli certificate sign account_key.prv | tee stake_delegation.cert
cert1q8rv4ccl54k99rtnm39...zr0
```

The output can now be added in the `transaction` and submitted to a node.

## submitting to a node

The to `jcli transaction add-certificate` command can be used to add a certificate to a transaction in _finalized_ state.

For example:

```sh

...

jcli transaction add-certificate $(cat stake_delegation.cert) --staging tx

jcli transaction finalize CHANGE_ADDRESS --fee-constant 5 --fee-coefficient 2 --fee-certificate 2 --staging tx

...

```

The `--fee-certificate` flag indicates the cost of adding a certificate, used for computing the fees, it can be omitted if it is zero.

See [here](../jcli/transaction.md) for more documentation on transaction creation.
