# Certificate

Tooling for offline transaction creation

## Building stake pool registration certificate

Builds a signed certificate.

The process can be split into steps on the first step certificate
is created.
```sh
jcli certificate new stake-pool-registration \
  --vrf-key <vrf-public-key> --kes-key <kes-public-key> \
  [--owner <owner-public-key>] \
  --serial <node-serial> \
  <output-file>
```

if output-file is omited result will be written to stdout. Once
certificate is ready you must sign it with the private keys of
all the owners:

```sh
jcli certificate sign <key> <input-file> <output-file>
```

## Building stake pool delegation certificate

Builds a stake pool delegation certificate

```sh
jcli certificate new stake-delegation [OPTIONS] <STAKE_KEY> <STAKE_POOL_IDS>...
```

Options are:
-o, --output <output> - write the output to the given file or print it to the standard output if not defined
<STAKE_KEY>           - the public key used in the stake key registration
<STAKE_POOL_IDS>...   - hex-encoded stake pool IDs and their numeric weights in format "pool_id:weight". If weight is not provided, it defaults to 1.
