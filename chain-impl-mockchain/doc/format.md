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

* Size of Header: 2 bytes (16 bits): Maximum header is thus 64K not including the block content
* Version of block: 2 bytes (16 bits)
* Size of Content: 4 bytes (32 bits)
* Block Date: Epoch (4 bytes, 32 bits) + Slot-id (4 bytes - 32 bits)
* Chain length (number of ancestor blocks; first block has chain length 0): 4 bytes (32 bits)
* Hash of content `H(Content)` (32 bytes - 256 bits)
* Parent Header hash : 32 bytes (256 bits)

We reserved the special value of all 0 for the parent header hash, to
represent the lack of parent for the block0, but for other blocks it's not
reserved and could represent, although with negligeable probability, a valid
block. In any case, it means that there's no special meaning to this value in
normal context.

In BFT the header also contains (768 bits = 96 bytes):

* BFT Public Key of the leader (32 bytes)
* BFT Signature (64 bytes)

In Praos/Genesis the header also contains (616 bytes):

* VRF PubKey: 32 bytes (ristretto25519)
* VRF Proof: 96 bytes (ristretto25519 DLEQs)
* KES Signature: 484 bytes (sumed25519-12)

Additionally, we introduce the capability to address each header individually
by using a cryptographic hash function : `H(HEADER)`. The hash include all
the content serialized in the sequence above, except the size of header,
which effectively means that calculating the hash of a fully serialized
header is just applying the hash function to the binary data except the first
2 bytes.

## Block Body

We need to be able to have different type of content on the blockchain, we also
need a flexible system for future expansion of this content.  The block content
is effectively a sequence of serialized content, one after another.

Each individual piece of block content is called a fragment and is prefixed
with a header which contains the following information:

* Size of content piece in bytes (2 bytes)
* Type of piece (1 byte): up to 256 different type of block content.

The block body is formed of the following stream of data:

    HEADER(FRAGMENT1) | FRAGMENT1 | HEADER(FRAGMENT2) | FRAGMENT2 ...

Where HEADER is:

	SIZE (2 bytes) | TYPE (1 byte)

Additionally, we introduce the capability to refer to each fragment
individually by fragment-id, using a cryptographic hash function : `H(TYPE |
CONTENT)`. The hash doesn't include the size prefix in the header to simplify
calculation of hash with on-the-fly (non serialized) structure.

Types of content:

* Transaction
* Old Transaction
* Certificate (Staking, Pool, Delegation, ...)
* TBD Update
* TBD Debug Stats : block debug information in chain.

### Common Structure

#### Token Transfer

Token Transfer is found in different type of messages, and allow to transfer tokens between an input to an output.

There's 4 differents type of supported spending:

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

## Type 0: Initial blockchain configuration

This message type may only appear in the genesis block (block 0) and
specifies various configuration parameters of the blockchain. Some of
these are immutable, while other may be changed via the update
mechanism (see below). The format of this message is:

    ConfigParams

where `ConfigParams` consists of a 16-bit field denoting the number of
parameters, followed by those parameters:

    Length | ConfigParam*{Length}

`ConfigParam` has the format:

    TagLen Payload

where `TagLen` is a 16-bit bitfield that has the size of the payload
(i.e. the value of the parameter) in bytes in the 6 least-significant
bits, and the type of the parameter in the 12 most-significant
bits. Note that this means that the payload cannot be longer than 63
bytes.

The following parameter types exist:

| tag  | name                                 | value type | description                                                                            |
| :--- | :----------------------------------- | :--------- | :------------------------------------------------------------------------------------- |
| 1    | discrimination                       | u8         | address discrimination; 1 for production, 2 for testing                                |
| 2    | block0-date                          | u64        | the official start time of the blockchain, in seconds since the Unix epoch             |
| 3    | consensus                            | u16        | consensus version; 1 for BFT, 2 for Genesis Praos                                      |
| 4    | slots-per-epoch                      | u32        | number of slots in an epoch                                                            |
| 5    | slot-duration                        | u8         | slot duration in seconds                                                               |
| 6    | epoch-stability-depth                | u32        | the length of the suffix of the chain (in blocks) considered unstable                  |
| 8    | genesis-praos-param-f                | Milli      | determines maximum probability of a stakeholder being elected as leader in a slot      |
| 9    | max-number-of-transactions-per-block | u32        | maximum number of transactions in a block                                              |
| 10   | bft-slots-ratio                      | Milli      | fraction of blocks to be created by BFT leaders                                        |
| 11   | add-bft-leader                       | LeaderId   | add a BFT leader                                                                       |
| 12   | remove-bft-leader                    | LeaderId   | remove a BFT leader                                                                    |
| 13   | allow-account-creation               | bool (u8)  | 0 to enable account creation, 1 to disable                                             |
| 14   | linear-fee                           | LinearFee  | coefficients for fee calculations                                                      |
| 15   | proposal-expiration                  | u32        | number of epochs until an update proposal expires                                      |
| 16   | kes-update-speed                     | u32        | maximum number of seconds per update for KES keys known by the system after start time |

`Milli` is a 64-bit entity that encoded a non-negative, fixed-point
number with a scaling factor of 1000. That is, the number 1.234 is
represented as the 64-bit unsigned integer 1234.

`LinearFee` has the format:

    Constant | Coefficient | Certificate

all of them 64-bit unsigned integers, specifying how fees are computed
using the formula:

    Constant + Coefficient * (inputs + outputs) + Certificate * certificates

where `inputs`, `outputs` and `certificates` represent the size of the
serialization of the corresponding parts of a transaction in bytes.

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

`Proposal` has the following format:

    ConfigParams

where `ConfigParams` is defined above.

## Type 5: Update votes

Vote messages register a positive vote for an earlier update
proposal. They have the format

    ProposalId | VoterId | Signature

where `ProposalId` is the message ID of an earlier update proposal
message, `VoterId` is an ed25519 extended public key, and `Signature`
is a signature by the corresponding secret key over `ProposalId |
VoterId`.
