# Transaction

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
2. `lock` the transaction for signing:
3. create witnesses and add the witnesses:
    - `make-witness`
    - `add-witness`
4. `finalize` the transaction, ready to send to the blockchain

There are also functions to help decode and display the
content information of a transaction:

* `info`
* `id` to get the **Transaction ID** of the transaction
