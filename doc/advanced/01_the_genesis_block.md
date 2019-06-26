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
{{#include ../../jormungandr-lib/src/interfaces/block0_configuration/DOCUMENTED_EXAMPLE.yaml}}
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
| `linear_fees` | object | linear fee settings, set the fee for transaction and certificate publishing |
| `consensus_genesis_praos_active_slot_coeff` | number | genesis praos active slot coefficient.  Determines minimum stake required to try becoming slot leader, must be in range (0,1] |
| `kes_update_speed` | number | the speed to update the KES Key in seconds |
| `slots_per_epoch` | number | number of slots in each epoch |

_for more information about the BFT leaders in the genesis file, see
[Starting a BFT Blockchain](./02_starting_bft_blockchain.md)_

## `initial` options

Each entry can be one of 3 variants:

| variant | format | description |
|:-------|:-------|:------------|
| `fund` | sequence | initial deposits present in the blockchain (up to 255 outputs per entry) |
| `cert` | string | initial certificate |
| `legacy_fund` | sequence | same as `fund`, but with legacy Cardano address format |

Example:

```yaml
initial:
  - fund:
      - address: <address>
        value: 10000
      - address: <address2>
        value: 20000
      - address: <address3>
        value: 30000
  - cert: <certificate>
  - legacy_fund:
      - address: <legacy address>
        value: 123
  - fund:
      - address: <another address>
        value: 1001
```

### `fund` and `legacy_fund` format

| variant | format | description |
|:-------|:-------|:------------|
| `address` | string | can be a [single address](../jcli/address.md#address-for-utxo) or an [account address](../jcli/address.md#address-for-account) |
| `value` | number | assigned value |

`legacy_fund` differs only in address format, which is legacy Cardano
