# Address

This is used to uniquely address an entity of the network. The domain is open and non enumerable.

The address format satisfies the following properties:

* Compact on the blockchain
* Extendable to new types
* Doesn't require any 3rd parties functionality, library or serialization format
* Easy to use by 3rd parties, hw modules, web interfaces, etc.
* Overall variable size but fixed related to each type
* Allow to distinguish testnet(s) from mainnet(s)

Discriminant allow to distinguish between mainnet and non-mainnet, but apart from this bit, no
extra information is used to distinguish between networks.

4 kind of addresses are currently supported:

* Single: Just a spending public key
* Group: Same as single, but with an added (staking/group) public key
  using the ED25519 algorithm.
* Account: An account public key using the ED25519 algorithm
* Multisig: A multisig account public key

## Human encoding

While we don't enforce any human encoding in the protocol,
for other human encoding/interaction (tools, UI), BECH32 is
the recommended options:

* easy human readable encoding (similar to base32)
* prefix
* checksum to detect errors

The blockchain network doesn't use any human encoding in its data, and
thus the BECH32 prefix is left to clients / networks. It's also perfectly
possible technically for different clients to use different prefix for the same
networks although it would be confusing to the user of this network.

When human interaction with serialized address is required, base32 can be used.

## Encoding

It uses a simple serialization format which is made to be concise:

* First byte contains the discrimination information (1 bit) and the kind of address (7 bits)
* Remaining bytes contains a kind specific encoding describe after.

The first byte is encoded as such:

    DISCRIMINATION_BIT (highest bit) || TYPE (lowest 7 bits)

        MSB     LSB
         xyyy yyyy
         | \___\___
         |         \_ type (7 bits)
         |
          \__ discrimination


Discrimininant bit is defined as:

* TestNet type of address : 0x1
* MainNet type of address : 0x0

The following types values are currently used:

* Reserved (do not use): 0x0, 0x1, 0x2
* Single: 0x3
* Group: 0x4
* Account: 0x5
* Multisig: 0x6

## Known type of payloads

Single type:

    SPENDING_KEY (ED25519 Public Key - 32 bytes)

Group type:

    SPENDING_KEY (ED25519 Public Key - 32 Bytes) || ACCOUNT_KEY (ED25519 Public Key - 32 Bytes)

Account type:

    ACCOUNT_KEY (ED25519 Public Key - 32 Bytes)

Multisignature type:

    MULTISIG_MERKLE_ROOT_PUBLIC_KEY (32 Bytes)
