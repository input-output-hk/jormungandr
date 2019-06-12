# delegating your stake

Now that you have created your [`stake key`] you can choose
to delegate your staking power to a stake pool.

## how to create the delegation certificate

You will need your:

* `stake_key.pub` file as created in [`stake key`];
* the Stake Pool ID: 32bytes identifying the stake pool you want
  to delegate your stake to.

```
$ jcli certificate new stake-delegation \
    ea830e5d9647af89a5e9a4d4089e6e855891a533316adf4a42b7bf1372389b74 \
    $(cat stake_key.pub) > stake_delegation.cert
```

## how to sign your delegation certificate

Just like for the [`stake key`] certificate:

```
$ cat stake_delegation.cert| jcli certificate sign stake_key.prv | tee stake_delegation.cert
cert1q8rv4ccl54k99rtnm39...zr0
```

The output can now be added in the `transaction` and submitted to a node.

[`stake key`]: ./registering_stake.md

## submitting to a node

To `jcli transaction add-certificate` command can be used to add a certificate to a transaction in _finalized_ state.

For example:

Note that in order to finalize a transaction, it should have both inputs and outputs.

```sh

...

jcli transaction finalize CHANGE_ADDRESS --fee-constant 5 --fee-coefficient 2 --fee-certificate 2 --staging tx

jcli transaction add-certificate $(cat stake_delegation.cert) --staging tx

...

```

The `--fee-certificate` flag indicates the cost of adding a certificate, used for computing the fees, it can be omitted if it is zero.

See [here](../jcli/transaction.md) for more documentation on transaction creation.