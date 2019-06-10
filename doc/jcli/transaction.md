# transaction

Tooling for offline transaction creation and signing.

```
jcli transaction
```

Those familiar with [`cardano-cli`](http://github.com/input-output-hk/cardano-cli)
transaction builder will see resemblance in `jcli transaction`.

There is a couple of commands that can be used to:

1. prepare a transaction:
    - `new` create a new empty transaction;
    - `add-input`
    - `add-account`
    - `add-output`
2. `finalize` the transaction for signing:
3. create witnesses and add the witnesses:
    - `make-witness`
    - `add-witness`
4. `seal` the transaction, ready to send to the blockchain

There are also functions to help decode and display the
content information of a transaction:

* `info`
* `id` to get the **Transaction ID** of the transaction
* `to-message` to get the hexadecimal encoded message, ready to send with `cli rest message`


# Examples

Let's say we have the following utxo

```plaintext
in_idx: 0
in_txid: 55762218e5737603e6d27d36c8aacf8fcd16406e820361a8ac65c7dc663f6d1c
out_addr: ta1sn9u0nxmnfg7af8ycuygx57p5xgzmnmgtaeer9xun7hly6mlgt3pjp0h0rrq782p7z4auve0s4l72egyc88f49mzm8lwuaw7xnw52udky79t3s
out_value: 100
```

And we want to transfer 50 lovelaces to the following address

**ta1ssnr5pvt9e5p009strshxndrsx5etcentslp2rwj6csm8sfk24a2wu27hujgyzl4mlc736jaztykud5tqrxw5gd6esmw7rx5zujdzw3ahcaskk**

***

**Note**: 

The following examples assume that you have something like this in your node configuration file (and that you have a locally running node).

You can check the [rest section](./rest.md) for more information on the rest commands.

```yaml
rest:
  listen: "127.0.0.1:8443"
  prefix: "api"
```

For example, for getting all the utxos:
`jcli rest v0 utxo get --host http://127.0.0.1:8443/api`

***

## Create a staging area

```sh
jcli transaction new > tx
```

## Add input

For the input, we need to reference the uxto with the **in_txid** and **in_idx** fields and we need to specify how much coins are there with the **out_value** field.

### Example

```sh
jcli transaction add-input  55762218e5737603e6d27d36c8aacf8fcd16406e820361a8ac65c7dc663f6d1c 0 100 --staging tx
```

## Add output

For the output, we need the address we want to transfer to, and the amount.

```sh
jcli transaction add-output ta1ssnr5pvt9e5p009strshxndrsx5etcentslp2rwj6csm8sfk24a2wu27hujgyzl4mlc736jaztykud5tqrxw5gd6esmw7rx5zujdzw3ahcaskk 50 --staging tx
```

## Add fee and change address

We want to get the change in the same address that we are sending from. We also specify how to compute the fees.
You can leave out the `-fee-constant 5 --fee-coefficient 2` part if those are both 0.

```sh
jcli transaction finalize ta1sn9u0nxmnfg7af8ycuygx57p5xgzmnmgtaeer9xun7hly6mlgt3pjp0h0rrq782p7z4auve0s4l72egyc88f49mzm8lwuaw7xnw52udky79t3s --fee-constant 5 --fee-coefficient 2 --staging tx
```

Now, if you run

```sh
jcli transaction info --fee-constant 5 --fee-coefficient 2 --staging tx
```

You should see something like this

```plaintext
Transaction `0df39a87d3f18a188b40ba8c203f85f37af665df229fb4821e477f6998864273' (finalizing)
  Input:   100
  Output:  89
  Fees:    11
  Balance: 0
 - 55762218e5737603e6d27d36c8aacf8fcd16406e820361a8ac65c7dc663f6d1c:0 100
 + ta1ssnr5pvt9e5p009strshxndrsx5etcentslp2rwj6csm8sfk24a2wu27hujgyzl4mlc736jaztykud5tqrxw5gd6esmw7rx5zujdzw3ahcaskk 50
 + ta1sn9u0nxmnfg7af8ycuygx57p5xgzmnmgtaeer9xun7hly6mlgt3pjp0h0rrq782p7z4auve0s4l72egyc88f49mzm8lwuaw7xnw52udky79t3s 39
```

## Sign the transaction

### Make witness

For signing the transaction, you need the private key associated with the input address (the one that's in the utxos) and the hash of the genesis block of the network you are connected to.

The following command takes the private key in the *key.prv* file and creates a witness in a file named *witness* in the current directory. 

```sh
jcli transaction make-witness --genesis-block-hash abcdef987654321... --type utxo txid --staging tx witness key.prv
```

### Add witness

```sh
jcli transaction add-witness witness --staging tx
```

## Send the transaction

```sh
jcli transaction seal --staging tx
```

```sh
jcli transaction to-message --staging tx > txmsg
```

Send it using the rest api

```sh
jcli rest v0 message post -f txmsg --host http://127.0.0.1:8443/api
```

## Checking if the transaction was accepted

You can check if the transaction was accepted by checking the node logs, for example

`jcli rest v0 message logs -h http://127.0.0.1:8443/api`

```plaintext
---
- fragment_id: d6ef0b2148a51ed64531efc17978a527fd2d2584da1e344a35ad12bf5460a7e2
  last_updated_at: "2019-06-11T15:38:17.070162114Z"
  received_at: "2019-06-11T15:37:09.469101162Z"
  received_from: Rest
  status:
    InABlock: "4.707"
```