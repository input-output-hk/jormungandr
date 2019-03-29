# Genesis File

The genesis file is the file that allows you to create a new blockchain
from block 0. It lays out the different parameter of your blockchain:
the initial utxo, the start time, the slot duration time, etc...

Example of a BFT genesis file with an initial address UTxO and an account UTxO.
More info regarding [starting a BFT blockchain here](./starting_bft_blockchain.md)
and [regarding addresses there](./cli_address.md).

You can generate a documented pre-generated genesis file:

```
jcli genesis init
```

```yaml
blockchain_configuration:
  - [ block0-date, 1552990378 ]
  - [ discrimination, test ]
initial_setting:
  max_number_of_transactions_per_block: 255
  bootstrap_key_slots_percentage: ~
  slot_duration: 15
  epoch_stability_depth: 10
  consensus: bft
  bft_leaders:
    - ed25519extended_public1k3wjgdcdcn23k6dwr0cyh88ad7a4ayenyxaherfazwy363pyy8wqppn7j3
    - ed25519extended_public13talprd9grgaqzs42mkm0x2xek5wf9mdf0eefdy8a6dk5grka2gstrp3en
  allow_account_creation: true
  linear_fee:
    constant: 2
    coefficient: 1
    certificate: 4
initial_utxos:
  - address: ta1svy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvxlswdf0
    value: 10000
legacy_utxos: ~
```

There are multiple _parts_ in the genesis file:

* `blockchain_configuration`: this is the static setting of the blockchain
  the data the cannot change, ever.
* `initial_setting`: this is a list of settings that can be change later
  utilising the update protocol.
* `initial_utxos`: the list of initial utxos (addresses and credited value);
* `legacy_utxos`: the list of legacy cardano utxos (base58 encoded addresses
  and credited values);

### `blockchain_configuration` options

| option | format | description |
|:-------|:-------|:------------|
| `block0-date` | number | the official start time of the blockchain, in seconds since UNIX EPOCH |
| `discrimination` | string | `production` or `test` |

### initial settings

| option | format | description |
|:-------|:-------|:------------|
| `max_number_of_transactions_per_block` | number | the maximum number of transactions allowed in a block |
| `bootstrap_key_slots_percentage` | number | placeholder, do not use |
| `slot_duration` | number | the number of seconds between the creation of 2 blocks |
| `epoch_stability_depth` | number | allowed size of a fork (in number of block) |
| `consensus` | string | the consensus version at the startup of the blockchain (`bft` for BFT) |
| `allow_account_creation` | boolean | allow creating accounts without publishing certificate |
| `linear_fee` | object | linear fee settings, set the fee for transaction and certificate publishing |
| `bft_leaders` | array | the list of the BFT leader at the beginning of the blockchain |

_for more information about the BFT leaders in the genesis file, see
[Starting a BFT Blockchain](./starting_bft_blockchain.md)_

### The initial UTxO

This is a list of the initial token present in the blockchain. It can be:

* classic UTxO: a [single address](./cli_address.md#address-for-utxo) and a value
* an account (if `allow_account_creation` is set to true): an
  [account address](./cli_address.md#address-for-account) and a value

### The legacy UTxO

This is a list of legacy cardano addresses and associated credited value.

Example:

```yaml
legacy_utxos:
  - address: Ae2tdPwUPEZCEhYAUVU7evPfQCJjyuwM6n81x6hSjU9TBMSy2YwZEVydssL
    value: 2000
```
