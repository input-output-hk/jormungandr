# genesis file

The genesis file is the file that allows you to create a new blockchain
from block 0. It lays out the different parameter of your blockchain:
the initial utxo, the start time, the slot duration time, etc...

Example of a BFT genesis file with an initial address UTxO and an account UTxO.
More info regarding [starting a BFT blockchain here](./02_starting_bft_blockchain.md)
and [regarding addresses there](../jcli/address.md).
You could also find information regarding the [jcli genesis tooling](../jcli/genesis.md).

You can generate a documented pre-generated genesis file:

```
jcli genesis init
```

For example your genesis file may look like:

```yaml
{{#include ../../src/bin/jcli_app/block/DOCUMENTED_EXAMPLE.yaml}}
```

There are multiple _parts_ in the genesis file:

* `blockchain_configuration`: this is a list of configuration
  parameters of the blockchain, some of which can be changed later
  via the update protocol;
* `initial_utxos`: the list of initial utxos (addresses and credited value);
* `legacy_utxos`: the list of legacy cardano utxos (base58 encoded addresses
  and credited values).

### `blockchain_configuration` options

| option | format | description |
|:-------|:-------|:------------|
| `block0_date` | number | the official start time of the blockchain, in seconds since UNIX EPOCH |
| `discrimination` | string | `production` or `test` |
| `block0_consensus` | string | `bft` |
| `slot_duration` | number | the number of seconds between the creation of 2 blocks |
| `epoch_stability_depth` | number | allowed size of a fork (in number of block) |
| `consensus_leader_ids` | array | the list of the BFT leader at the beginning of the blockchain |
| `max_number_of_transactions_per_block` | number | the maximum number of transactions allowed in a block |
| `bft_slots_ratio` | number | placeholder, do not use |
| `allow_account_creation` | boolean | allow creating accounts without publishing certificate |
| `linear_fee` | object | linear fee settings, set the fee for transaction and certificate publishing |

_for more information about the BFT leaders in the genesis file, see
[Starting a BFT Blockchain](./02_starting_bft_blockchain.md)_

### The initial Funds

This is a list of the initial token present in the blockchain. It can be:

* classic UTxO: a [single address](../jcli/address.md#address-for-utxo) and a value
* an account (if `allow_account_creation` is set to true): an
  [account address](../jcli/address.md#address-for-account) and a value

### The legacy Funds

This is a list of legacy cardano addresses and associated credited value.

Example:

```yaml
legacy_funds:
  - address: Ae2tdPwUPEZCEhYAUVU7evPfQCJjyuwM6n81x6hSjU9TBMSy2YwZEVydssL
    value: 2000
```
