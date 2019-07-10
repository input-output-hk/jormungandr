# Multisig

This document defines the extension to the current protocol to support multisig accounts.
Multisig accounts are similar to normal account, except they are controlled by 2 or more
parties, and have a threshold for operation.

For example some use cases of account purpose:

* Alice, and Bob have an account when both their signatures are needed to do any operation.
* Alice, Bob and Charles have an account where any valid 2 out of 3 of their signatures is required to do any operation.
* The CEO, CFO, and one of 3 engineering directors are needed to do any operation.
* The CEO, or the CFO, or 2 of the 3 engineering directors are needed to do any operation.

## Communication

This documentation doesn't cover the out-of-band communication that need to happens
between parties of a multisig account, for registration and creation of transactions.

We assume participants a multisig account, are able to communicate on a channel.
The channel should be secure against tempering related to the endpoint, but
doesn't require any privacy to be secure. i.e. communication can be
eavedropped, but the data should be authenticated between parties.

Note that this requirement is not as strong during transaction creation,
other mechanisms should catch issues related to malicious parties.

## Threshold Identification Merkle Tree (TIMT)

We use the following primitive construction to describe a basic multisig elements

* Threshold: an unsigned integer between 1 to the number of keys.
* A sequence of public participant of this tree, which are either a public key or a TIMT Identifier for recursive scheme

Effectively:

    TIMT<N> = Threshold x [Type x Hash; N]

We construct a unique identifier out of the following:

    Identifier(TIMT<N>) = H(TIMT.Threshold | TIMT.Hash[0] | .. | TIMT.Hash[N-1])

To construct a valid witness related to this construction, one need to
provide at least minimum of Threshold signature elements for a given message with their respective
indices:

    sign(TIMT<N>, Message) = [ Index x Signature[Index]; E ]
       where TIMT.Threshold <= E <= TIMT.N

To validate we need to make sure that the number of participants is correct
for each level, validating all cryptographic signatures for non-TIMT participants,
and recursively re-building the identifier of the TIMT.

    validate(signatures, TIMT<N>, Message) =
         TIMT.Threshold <= #signatures <= TIMT.N
      && for all { signatures where signature.type is not TIMT }:
            validate(sig, message) == true
      && Identifier(TIMT) == Identifier(fill_or_replace(TIMT, signature))

    fill_or_replace(signature, TIMT<N>) =
        For i in TIMT.N:
            if signature.index == Present:
                TIMT.hash[I] = signature.sighash

The individual signature is of the form:

    sign(individual, message) =
        individual.public-key x individual.sign(individual.secret, message)

## Registration

Multisig accounts can be created using a multisig registration certificate.
The certificate contains one to multiple TIMT. Each TIMT need to be individually valid.

Some limits of the maximum size of individual elements, and the overall scheme need
to be enforced.

Practically, the scheme becomes already complicated from a UX point of view
when reaching 2 levels, so we enforce an arbitrary limit of 2 levels, and that
each tree should have a maximum of 8 participants.

## Identification

The Identification of a multisig account is the Identifier of the toplevel TIMT.
A special identifier for multisig account is reserved in the address scheme to address
multisig account and differentiate them from normal account.

TODO: Add format

## Transaction

A special multisig witness is added to transaction input validation. Each individual bits
of the witness needs to be ordered by their respective tree index.

TODO: Add format

## Staking & Ownership

Multisig account can, just like normal accounts (group key), have their stake
delegated through the stake delegation certificate, and register stake pool as
one of the owner. The same mechanism used for transaction witnessing is
used 