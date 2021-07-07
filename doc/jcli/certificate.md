# Certificate

Tooling for offline transaction creation

## Building stake pool registration certificate

Builds a stake pool registration certificate.

```sh
jcli certificate new stake-pool-registration \
    --vrf-key <vrf-public-key> \
    --kes-key <kes-public-key> \
    --start-validity <seconds-since-start> \
    --management-threshold <THRESHOLD> \
    --owner <owner-public-key> \
    [--operator <operator-public-key>] \
    [<output-file>]
```

Where:

- `--operator <operator-public-key>` - *optional*, public key of the operator(s) of the pool.
- `output-file`                      - *optional*, write the output to the given file or print it to the standard output if not defined

## Retiring a stake pool

It is possible to retire a stake pool from the blockchain. By doing so the stake delegated
to the stake pool will become dangling and will need to be re-delegated.

Remember though that the action won't be applied until the next following epoch. I.e.
the certificate will take a whole epoch before being applied, this should leave time
for stakers to redistribute their stake to other pools before having their stake
becoming dangling.

It might be valuable for a stake pool operator to keep the stake pool running until
the stake pool retirement certificate is fully applied in order to not miss any
potential rewards.

example:

```sh
jcli certificate new stake-pool-retirement \
    --pool-id <STAKE_POOL_ID> \
    --retirement-time <seconds-since-start> \
    [<output-file>]
```

where:

- `output-file`                 - *optional*, write the output to the given file
                                  or print it to the standard output if not defined.
- `--retirement-time`           - is the number of seconds since the start in order
                                  to make the stake pool retire. `0` means as soon as possible.
- `--pool-id`                   - hex-encoded stake pool ID. Can be retrieved using  `jcli certificate get-stake-pool-id` command.
                                  See [here](../stake_pool/registering_stake_pool.md) for more details.

## Building stake pool delegation certificate

Builds a stake pool delegation certificate.

```sh
jcli certificate new stake-delegation <STAKE_KEY> <STAKE_POOL_IDS> [--output <output-file>]
```

Where:

- `-o, --output <output-file>` - *optional*, write the output to the given file or print it to the standard output if not defined
- `<STAKE_KEY>`                - the public key used in the stake key registration
- `<STAKE_POOL_IDS>...`        - hex-encoded stake pool IDs and their numeric weights in format **"pool_id:weight"**.
                                 If *weight* is not provided, *it defaults to 1*.
