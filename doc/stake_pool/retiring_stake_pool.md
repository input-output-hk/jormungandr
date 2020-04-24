# Retiring a stake pool

Stake pool can be retired by sending transaction with retirement certificate.
From technical stand point, it is very similar to register stake pool operation.
Before start we need to be sure, that:

* you have sufficient amount of ada to pay fee for transaction with retirement certificate.
* you know your stake pool id.

## Retrieve stake pool id

To retrieve your stake pool id:

```sh
jcli certificate get-stake-pool-id stake_pool.cert
ea830e5d9647af89a5e9a4d4089e6e855891a533316adf4a42b7bf1372389b74
```

### creating a retirement certificate

The certificate is what will be sent to the blockchain in order to retire
your stake pool.

```sh
jcli certificate new stake-pool-retirement \
    --pool-id ea830e5d9647af89a5e9a4d4089e6e855891a533316adf4a42b7bf1372389b74 \
    --retirement-time 0 \
    retirement.cert
```

where:

- `retirement.cert`                                                             - write the output of to the `retirement.cert`
- `--retirement-time 0 `                                                        - `0` means as soon as possible. Which is until the next following epoch.
- `--pool-id ea830e5d9647af89a5e9a4d4089e6e855891a533316adf4a42b7bf1372389b74`  - hex-encoded stake pool ID.

### submitting to a node

The `jcli transaction add-certificate` command should be used to add a certificate **before finalizing** the transaction.

For example:

```sh
...

jcli transaction add-certificate $(cat retirement.cert) --staging tx
jcli transaction finalize CHANGE_ADDRESS --fee-constant 5 --fee-coefficient 2 --fee-certificate 2 --staging tx

...
jcli transaction seal --staging tx
jcli transaction auth --key owner_key.prv --staging tx
...
```

The `--fee-certificate` flag indicates the cost of adding a certificate, used for computing the fees, it can be omitted if it is zero.

**Important !**
Please be sure that you have sufficient amount of owners signatures in order to retire stake pool. At least half of owners singatures (which were provided when registering stake pool) are required to sign retirement certificate.

See [here](../jcli/transaction.md) for more documentation on transaction creation.
