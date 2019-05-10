**This is a draft document**

# Preliminaries

All integers are encoded in big-endian format.

`Signature` has the format

    Length | Payload

where `Length` is a 16-bit unsigned integer `N`, and `Payload` is `N`
bytes of signature data.

# Block

Format is:

    Header | Content

## Block Header

The header is a small piece of data, containing enough informations for validation and network deduplication and a strong signed cryptographic link to the content.

Common (2 * 64 bits + 1 * 32 bits + 2 * 256 bits = 84 bytes):

* Size of Header: 16 bits: Maximum header is thus 64K not including the block content
* Version of block: 16 bits
* Size of Content: 32 bits
* Block Date: Epoch (32 bits) + Slot-id (32 bits)
* Chain length (number of ancestor blocks; first block has chain length 0): 32 bits
* Hash of content `H(Content)` (256 bits)
* Parent Header hash : 256 bits (with the special value of 0 to represent the lack of parent for the first block)

In BFT the header also contains (768 bits = 96 bytes):

* BFT Public Key of the leader (256 bits)
* BFT Signature (512 bits)

In Praos/Genesis the header also contains (128 bytes + between 480 to 1184 bytes = between 608 to 1312 bytes):

* VRF PubKey: 256 bits (curve25519-dalek)
* VRF Proof: 768 bits (curve25519-dalek DLEQs)
* KES Signature (content TBD)
* MMM+ed25519: Between 480 Bytes <=> 1184 Bytes

Additionally, we introduce the capability to address each header individually
by using a cryptographic hash function : `H(HEADER)`. The hash include all
the content serialized in the sequence above, except the size of header,
which effectively means that calculating the hash of a fully serialized
header is just applying the hash function to the binary data except the first
2 bytes.

## Block Body

We need to be able to have different type of content on the blockchain, we
also need a flexible system for future extension. The block content is
effectively a sequence of serialized content, one after another.

Each individual piece of block content is prefixed with a header which
contains the following information:

* Size of content piece in bytes (2 bytes)
* Type of piece (1 byte): up to 256 different type of block content.

The block body is formed of the following stream of data:

    HEADER(CONTENT1) | CONTENT1 | HEADER(CONTENT2) | CONTENT2 ...

Where HEADER is:

	SIZE (2 bytes) | TYPE (1 byte)

Additionally, we introduce the capability to address each content object individually by using a cryptographic hash function : `H(TYPE | CONTENT)`. The hash doesn't include the size prefix in the header to simplify calculation of hash with on-the-fly (non serialized) structure.

Types of content:

* Transaction
* Old Transaction
* Certificate (Staking, Pool, Delegation, ...)
* TBD Update
* TBD Debug Stats : block debug information in chain.

### Common Structure

#### Token Transfer

Token Transfer is found in different type of messages, and allow to transfer tokens between an input to an output.

There's 4 differents type of spending:

* Utxo -> Utxo
* Utxo -> Account
* Account -> Utxo
* Account -> Account

We add support to this with the following TokenTransfer data structure:

* Transaction Header (2 bytes)
* Input number (1 byte: 256 inputs maximum)
* Output number (1 byte where 0xff is reserved: 255 outputs maximum)
* Transaction Inputs (Input number of time * 41 bytes):
  * Index (1 byte) : special value 0xff specify an account spending
  * Account Public Key or Transaction Hash (32 bytes) (which is H(CONTENT))
  * Value (8 bytes)
* Transaction Outputs (Output number of time):
  * Address (bootstrap address 33 bytes, delegation address 65 bytes, account address 33 bytes)
  * Value (8 bytes)

Value are encoded as fixed size 8 bytes, wasting a few bytes of space for small amounts, but making fee calculation simpler when based on bytes.

We add a way to refer to this content by hash using the following construction:

    H(HEADER | INPUTS | OUTPUTS)

Rationales:

* 1 byte index utxos: 256 utxos = 10496 bytes just for inputs, already quite big and above a potential 8K soft limit for block content
Utxo representation optimisations (e.g. fixed sized bitmap)

* Values in inputs:
Support for account spending: specifying exactly how much to spend from an account 
Light client don't have to trust the utxo information from a source (which can lead to e.g. spending more in fees), since a client will now sign a specific known value.

* Account Counter encoding:
4 bytes: 2^32 unique spending from the same account is not really reachable:
10 spending per second = 13 years to reach limit.
2^32 signatures on the same signature key is stretching the limits of scheme.
Just the publickey+witnesses for the maximum amount of spending would take 400 gigabytes

#### Witnesses

To authenticate such a data structure, we add witnesses with a 1-to-1 mapping
with inputs. The serialized sequence of inputs, is directly linked with the
serialized sequence of witnesses.

Fundamentally the witness is about signing a message and generating/revealing
cryptographic material to approve the unequivocally the content.

We have currently 3 differents types of witness that need support:

* Old address scheme: an extended public key, followed by the signature
* New address scheme: a signature
* Account witness

With the following serialization:

* Type of witness: 1 byte
* Then either:
  * Type=1 Old address witness scheme (128 bytes):
    * Extended Public key (64 bytes)
    * Signature (64 bytes)
  * Type=2 New address witness scheme (64 bytes):
    * Signature (64 bytes)
  * Type=3 Account witness (68 bytes):
    * Account Counter (4 bytes)
    * Signature (64 bytes)

The message, w.r.t the cryptographic signature, is generally of the form:

	Msg = HEADER | INPUTS | OUTPUTS | EXTRA

Where HEADER, INPUTS and OUTPUTS comes from the Token Transfer type, and EXTRA is the optional data serialized between the token transfer type, and the witnesses.

## Type 2: Transaction

Transaction is the composition of the TokenTransfer structure followed directly by the witnesses. EXTRA needs to be empty. Effectively:

    TokenTransfer | Witnesses

TODO:

* Multisig
* Fees

## Type 3: Certificate

Certificate is the composition of the TokenTransfer structure, followed by the certificate data, then the witnesses. Effectively:

    TokenTransfer | Certificate | Witnesses

Known Certificate types:

* Staking declaration: declare a staking key + account public information
* Stake pool registration: declare the VRF/KES key for a node.
* Delegation: contains a link from staking to stake pool.

Content:

* PublicKey
* Signature of the witness with the private key associated to the revealed PublicKey

## Type 4: Update Proposal

Update proposal messages propose new values for blockchain
settings. These can subsequently be voted on. They have the following
form:

    Proposal | ProposerId | Signature

where `ProposerId` is a ed25519 extended public key, and `Signature`
is a signature by the corresponding private key over the string
`Proposal | ProposerId`.

`Proposal` is a list of setting types and values, terminated by the
16-bit integer 0xffff:

    (SettingTag | SettingValue)* 0xffff

`SettingTag` is a 16-bit unsigned integer specifying the type of
setting, and `SettingValue` is a variable-length encoding of the
proposed value of the setting. The following setting types are
defined, with their corresponding values:

* 1: maximum number of transactions per block: 32-bit unsigned integer
* 2: 'd' parameter, i.e. percentage of slots that must be signed by
  BFT leaders: 8-bit integer in the range 0-100
* 3: consensus version: 16-bit unsigned integer
* 4: BFT leaders: 8-bit unsigned integer count of BFT leaders `N`,
  followed by `N` ed25519 public keys
* 5: whether account creation is allowed: a byte `0` for no, or `1`
  for yes
* 6: transaction linear fee: 64-bit summand, 64-bit multiplier, and a
  certificate (TODO)
* 7: slot duration: 8-bit unsigned integer
* 8: epoch stability depth: 32-bit unsigned integer

Settings must appear in monotonically increasing order of their tags,
that is, if tag N < M, then setting N must appear before M, and
settings cannot be repeated.

TODO: should `SettingValue` contain a length so we can at least parse
proposals containing unknown settings?

## Type 5: Update votes

Vote messages register a positive vote for an earlier update
proposal. They have the format

    ProposalId | VoterId | Signature

where `ProposalId` is the message ID of an earlier update proposal
message, `VoterId` is an ed25519 extended public key, and `Signature`
is a signature by the corresponding secret key over `ProposalId |
VoterId`.
