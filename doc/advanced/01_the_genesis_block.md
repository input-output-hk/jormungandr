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
* `initial`: list of steps to create initial state of ledger

## `blockchain_configuration` options

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

## `initial` options

Each entry can be one of 3 variants:

| variant | format | description |
|:-------|:-------|:------------|
| `fund` | object | initial deposits present in the blockchain |
| `cert` | string | initial certificate |
| `legacy_fund` | object| same as `fund`, but with legacy Cardano address format |

Example:

```yaml
initial:
  - fund:
      address: <address>
      value: 10000
  - cert: <certificate>
  - legacy_fund:
      address: <legacy address>
      value: 123
  - fund:
      address: <another address>
      value: 1001
```

### `fund` and `legacy_fund` format

| variant | format | description |
|:-------|:-------|:------------|
| `address` | string | can be a [single address](../jcli/address.md#address-for-utxo) or an [account address](../jcli/address.md#address-for-account) (if `allow_account_creation` is set to true) |
| `value` | number | assigned value |
