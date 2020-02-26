# Genesis

Tooling for working with a genesis file

# Usage
```sh
jcli genesis [subcommand]
```

## Subcommands

- decode: Print the YAML file corresponding to an encoded genesis block.
- encode: Create the genesis block of the blockchain from a given yaml file.
- hash: Print the block hash of the genesis 
- init: Create a default Genesis file with appropriate documentation to help creating the YAML file
- help

## Examples

### Encode a genesis file

```sh
jcli genesis encode --input genesis.yaml --output block-0.bin
```

or equivantely

```sh
cat genesis.yaml | jcli genesis encode > block-0.bin
```

### Get the hash of an encoded genesis file

```sh
jcli genesis hash --input block-0.bin
```