# Transaction builder

Builds a signed transaction

## Building

```
cargo build --bin tx_builder
```

## Usage

```
tx_builder <options>
```

The options are

- -f <value> or --fee-base <value> - fee which will be always added to the transaction
- -a <value> or --fee-per-addr <value> - fee which will be added to the transaction for every
input and output
- -i <input> or --input <input> - transaction input. Must have format
`<hex-encoded-transaction-id>:<output-index>:<value>`. E.g. `1234567890abcdef:2:535`.
At least 1 value required.
- -o <output> or --output <output> -transaction output. Must have format
`<hex-encoded-address>:<value>`. E.g. `abcdef1234567890:501`. At least 1 value required.
- -c <address> or --change <address> - change address. Value taken from inputs and not spent on
outputs or fees will be returned to this address. If not provided, the change will go to treasury.
Must be hex-encoded.
- -s <key> or --spending-key <key> - transaction spending keys. Must be hex-encoded.
Required as many as provided inputs.

Value outputted to stdout on success is binary blob with transaction
