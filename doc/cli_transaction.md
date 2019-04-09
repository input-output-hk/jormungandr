# Transaction

Tooling for offline transaction creation

## Builder

Builds a signed transaction message.

The process can be split into steps by passing --file parameter. The intermediate state
will be stored in the given file in YAML format or updated if it already exists. If
transaction is not supposed to be finalized yet, pass --draft flag.

### Usage

```
jcli transaction build <options>
```

The options are

FLAGS:
- -d, --draft do not generate final transaction
- -h, --help Prints help information
- -V, --version Prints version information

OPTIONS:
- -c, --change <change> change address. Value taken from inputs and not spent on outputs
or fees will be returned to this address. If not provided, the change will go to treasury.
Must be bech32-encoded ed25519e_pk key.
- -b, --fee-base <fee-base> fee base which will be always added to the transaction
- -a, --fee-per-addr <fee-per-addr> fee which will be added to the transaction for every
input and output
- -f, --file <file> create or update transaction builder state file
- -i, --input <input>... transaction input. Must have format
`<hex-encoded-transaction-id>:<output-index>:<value>`. E.g. `1234567890abcdef:2:535`.
- -o, --output <output>... transaction output. Must have format `<address>:<value>`.
E.g. `ed25519e_pk1abcdef1234567890:501`. The address must be bech32-encoded
ed25519extended_public key.
- -s, --spending-key <spending-key>... file with transaction spending keys. Must be
bech32-encoded ed25519e_sk. Required one for every input.

Value outputted to stdout on success is transaction message encoded as hex.


### Example

```
jcli transaction build --input f7f1b60d6033bf72409f5fca14fea21f657a0cee19729146351a698d2b6a853e:0:100  \
--output ta1sv5er2j6vpvhqsj0yy248tmdrxwz0c6u2uvays27ujqhk697dyk6csdzwvm:70 -s key.private
```
