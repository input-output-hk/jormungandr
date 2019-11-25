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
