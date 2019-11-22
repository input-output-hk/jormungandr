# Transaction

Tooling for offline transaction creation and signing.

```sh
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
5. `auth` the transaction, if it contains a certificate

There are also functions to help decode and display the
content information of a transaction:

- `info` displays summary of transaction being constructed
- `data-for-witness` get the data to sign from a given transaction
- `fragment-id` get the **Fragment ID** from a transaction in *sealed* state
- `to-message` to get the hexadecimal encoded message, ready to send with `cli rest message`

**DEPRECATED**:

- `id` get the data to sign from a given transaction (use `data-for-witness` instead)

## Transaction info

On every stage of building a transaction user can display its summary

```sh
jcli transaction info <options>
```

The options are:

- `--prefix <address-prefix>`       - set the address prefix to use when displaying the addresses (default: ca)

- `--fee-certificate <certificate>` - fee per certificate (default: 0)

- `--fee-coefficient <coefficient>` - fee per every input and output (default: 0)

- `--fee-constant <constant>`       - fee per transaction (default: 0)

- `--output-format <format>`        - Format of output data. Possible values: json, yaml.
                                      Any other value is treated as a custom format using values from output data structure.
                                      Syntax is Go text template: https://golang.org/pkg/text/template/. (default: yaml)

- `--output <output>`               - write the info in the given file or print it to the standard output

- `--staging <staging-file>`        - place where the transaction is going to be save during its staging phase.
                                      If a file is given, the transaction will be read from this file and modification will be written into this same file.
                                      If no file is given, the transaction will be read from the standard input and will be rendered in the standard output

YAML printed on success

```yaml
---
balance: 40         # transaction balance or how much input is not spent
fee: 60             # total fee for transaction
input: 200          # total input of transaction
inputs:             # list of transaction inputs, each can be of either "utxo" or "account" kind
  - index: 4        # index of transaction output
    kind: utxo      # constant value, signals that UTxO is used
                    # hex-encoded ID of transaction
    txid: 543326b2739356ab6d14624a536ca696f1020498b36456b7fdfe8344c084bfcf
    value: 130      # value of transaction output
  -                 # hex-encoded account address
    account: 3fd45a64ae5a3b9c35e37114baa099b8b01285f7d74b371597af22d5ff393d9f
    kind: account   # constant value, signals that account is used
    value: 70       # value taken from account
num_inputs: 1       # total number of inputs of transaction
num_outputs: 1      # total number of outputs of transaction
num_witnesses: 1    # total number of witnesses of transaction
output: 100         # total output of transaction
outputs:            # list of transaction outputs
  -                 # bech32-encoded address
    address: ca1swedukl830v26m8hl7e5dzrjp77yctuz79a68r8jl2l79qnpu3uwz0kg8az
    value: 100      # value sent to address
                    # hex-encoded transaction hash, when transaction is complete, it's also its ID
sign_data_hash: 26be0b8bd7e34efffb769864f00d7c4aab968760f663a7e0b3ce213c4b21651b
status: sealed      # transaction status, can be "balancing", "finalizing", "sealed" or "authed"
```

# Examples

The following example focuses on using an utxo as input, the few differences when transfering from an account will be pointed out when necessary.
There is also a script [here](https://github.com/input-output-hk/jormungandr/blob/master/scripts/send-transaction) to send a transaction from a faucet account to a specific address which could be used as a reference.

Let's use the following utxo as input and transfer 50 lovelaces to the destination address

## Input utxo

| Field                     | Value        |
| ------------------------- |:------------:|
| UTXO's transaction ID     | 55762218e5737603e6d27d36c8aacf8fcd16406e820361a8ac65c7dc663f6d1c|
| UTXO's output index       | 0     |
| associated address        |  ca1q09u0nxmnfg7af8ycuygx57p5xgzmnmgtaeer9xun7hly6mlgt3pjyknplu    |
| associated value          | 100             |

## Destination address

**address**: ca1qvnr5pvt9e5p009strshxndrsx5etcentslp2rwj6csm8sfk24a2wlqtdj6

## Create a staging area

```sh
jcli transaction new --staging tx
```

## Add input

For the input, we need to reference the uxto with the **UTXO's transaction ID** and **UTXO'S output index** fields and we need to specify how much coins are there with the **associated value** field.

### Example - UTXO address as Input

```sh
jcli transaction add-input 55762218e5737603e6d27d36c8aacf8fcd16406e820361a8ac65c7dc663f6d1c 0 100 --staging tx
```

### Example - Account address as Input

If the input is an account, the command is slightly different

```sh
jcli transaction add-account account_address account_funds --staging tx
```

## Add output

For the output, we need the address we want to transfer to, and the amount.

```sh
jcli transaction add-output ca1qvnr5pvt9e5p009strshxndrsx5etcentslp2rwj6csm8sfk24a2wlqtdj6 50 --staging tx
```

## Add fee and change address

We want to get the change in the same address that we are sending from (the *associated address* of the utxo). We also specify how to compute the fees.
You can leave out the `--fee-constant 5 --fee-coefficient 2` part if those are both 0.

```sh
jcli transaction finalize ca1q09u0nxmnfg7af8ycuygx57p5xgzmnmgtaeer9xun7hly6mlgt3pjyknplu --fee-constant 5 --fee-coefficient 2 --staging tx
```

Now, if you run

```sh
jcli transaction info --fee-constant 5 --fee-coefficient 2 --staging tx
```

You should see something like this

```yaml
---
balance: 0
fee: 11
input: 100
inputs:
  - index: 0
    kind: utxo
    txid: 55762218e5737603e6d27d36c8aacf8fcd16406e820361a8ac65c7dc663f6d1c
    value: 100
num_inputs: 1
num_outputs: 2
num_witnesses: 0
output: 89
outputs:
  - address: ca1qvnr5pvt9e5p009strshxndrsx5etcentslp2rwj6csm8sfk24a2wlqtdj6
    value: 50
  - address: ca1q09u0nxmnfg7af8ycuygx57p5xgzmnmgtaeer9xun7hly6mlgt3pjyknplu
    value: 39
sign_data_hash: 0df39a87d3f18a188b40ba8c203f85f37af665df229fb4821e477f6998864273
status: finalizing
```

## Sign the transaction

### Make witness

For signing the transaction, you need the private key associated with the input address (the one that's in the utxos) and the hash of the genesis block of the network you are connected to.

The genesis' hash is needed for ensuring that the transaction cannot be re-used in another blockchain and for security concerns on offline transaction signing, as we are signing the transaction for the specific blockchain started by this block0 hash.

The following command takes the private key in the *key.prv* file and creates a witness in a file named *witness* in the current directory.

```sh
jcli transaction make-witness --genesis-block-hash abcdef987654321... --type utxo txid witness key.prv
```

---

#### Account input

When using an account as input, the command takes `account` as the type and an additional parameter: `--account-spending-counter`, that should be increased every time the account is used as input.

e.g.

```sh
jcli transaction make-witness --genesis-block-hash abcdef987654321... --type account --account-spending-counter 0 witness key.prv
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

You can check if the transaction was accepted by checking the node logs, for example, if the transaction is accepted

`jcli rest v0 message logs -h http://127.0.0.1:8443/api`

```plaintext
---
- fragment_id: d6ef0b2148a51ed64531efc17978a527fd2d2584da1e344a35ad12bf5460a7e2
  last_updated_at: "2019-06-11T15:38:17.070162114Z"
  received_at: "2019-06-11T15:37:09.469101162Z"
  received_from: Rest
  status:
    InABlock:
      date: "4.707"
      block: "d9040ca57e513a36ecd3bb54207dfcd10682200929cad6ada46b521417964174"
```

Where the **InABlock** status means that the transaction was accepted in the block with date "4.707"
and for block `d9040ca57e513a36ecd3bb54207dfcd10682200929cad6ada46b521417964174`.

The status here could also be:

**Pending**: if the transaction is received and is pending being added in the blockchain (or rejected).

or

**Rejected**: with an attached message of the reason the transaction was rejected.
