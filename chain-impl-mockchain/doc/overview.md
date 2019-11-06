## Block

The Block is a Header followed by its associated Content.

```
    +-----------+
    | Header    |
    +-----------+
    | Contents  |
    +-----------+
```

## Header

The header contains blockchain organisation metadata (chain length, date, etc),
along with all the per-consensus metadata (VRF, KES, Signature).

It is refered using a cryptography unique identifier called the HeaderId, which
is computed on the whole serialized header **minus** its leading size.

The block **chain** component is handled by the parent-hash, which points
through the HeaderId to a unique header.

```
    
         HeaderId A    <-           HeaderId B
    +------------------+ \       +---------------+ 
    | depth=0          |  \      | depth=1       |
 |--| parent-hash=0*32 |   \-----| parent-hash=A |
    | ...              |         | ...           |
    +------------------+         +---------------+
```

The header also uniquely point to some content by means of a ContentId.

The header with depth=0, defines the "genesis block"

## Content

Content is composed of zero to many fragments appended one after another

ContentId is computed as the cryptographic hash of the whole content

```

    +---------------+
    | Fragment 1    |
    +---------------+
    | ....          |
    +---------------+
    | Fragment N    |
    +---------------+

```

## Fragment

Fragments are specific content that act on the state of the overall chain ledger.

Current fragments defined are :

* INITIAL: only found in genesis block, define the parameter of the blockchain
* OLD-UTXO-DECL: only found in genesis block, declares a set of old addresses with certain value, existing on this chain.
* SIMPLE-TRANSACTION: a spending transaction from some inputs to some outputs.
* OWNER-STAKE-DELEGATION: Establish the delegation setting of an account, where the fees are paid by the account.
* STAKE-DELEGATION: Establish the delegation settings of an account but fees are handled by a 3rd party.
* POOL-REGISTRATION: Register a new pool
* POOL-RETIREMENT: Retire a pool
* POOL-UPDATE: Update parameters of a pool
* UPDATE-PROPOSAL
* UPDATE-VOTE

The most common fragments are the transaction, and owner-stake-delegation.
