# Genesis File

The genesis file is the file that allows you to create a new blockchain
from block 0. It lays out the different parameter of your blockchain:
the initial utxo, the start time, the slot duration time, etc...

Example of a BFT genesis file with an initial address UTxO and an account UTxO.
More info regarding [starting a BFT blockchain here](./starting_bft_blockchain.md)
and [regarding addresses there](./cli_address.md).

```yaml
start_time: 1552990378
slot_duration: 15
epoch_stability_depth: 10
allow_account_creation: true
address_discrimination: Production
linear_fees:
  constant: 2
  coefficient: 1
  certificate: 4
initial_utxos:
  - address: ca1qvy0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvx5c3cy4
    value: 100
  - address: ca1q5y0mwwm7mdwcuj308aapjw6ra4c3e6cygd0f333nvtjzxg8ahdvx6g5gwu
    value: 10000
bft_leaders:
  - ed25519extended_public1k3wjgdcdcn23k6dwr0cyh88ad7a4ayenyxaherfazwy363pyy8wqppn7j3
  - ed25519extended_public13talprd9grgaqzs42mkm0x2xek5wf9mdf0eefdy8a6dk5grka2gstrp3en
```

| option | format | description |
|:-------|:-------|:------------|
| `start_time` | number | the official start time of the blockchain, in seconds since UNIX EPOCH |
| `slot_duration` | number | the number of seconds between the creation of 2 blocks |
| `epoch_stability_depth` | number | allowed size of a fork (in number of block) |
| `allow_account_creation` | boolean | allow creating accounts without publishing certificate |
| `address_discrimination` | String | `Production` or `Testing` |
| `linear_fee` | object | linear fee settings, set the fee for transaction and certificate publishing |
| `initial_utxos` | array | the list of initial UTxO |
| `bft_leaders` | array | the list of the BFT leader at the beginning of the blockchain |

## The initial UTxO

This is a list of the initial token present in the blockchain. It can be:

* classic UTxO: a [single address](./cli_address.md#address-for-utxo) and a value
* a legacy UTxO: (TBD)
* an account (if `allow_account_creation` is set to true): an
  [account address](./cli_address.md#address-for-account) and a value

## The BFT leaders

for more information about the BFT leaders in the genesis file, see
[Starting a BFT Blockchain](./starting_bft_blockchain.md)
