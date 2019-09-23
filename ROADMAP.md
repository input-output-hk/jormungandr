JORMUNGANDR ROADMAP
===================

Development major phases until "1.0":

1. Ledger
2. Network
3. Incentive
4. Stabilisation
5. Security audit
6. Performance improvements

However, the general order is not a strict order between phases,
since some tasks in other phases are started ahead of previous phases
as ressources and time permit.

Progress
--------

This is the general progress

| Phases           | Progress       |
| ---------------- | -------------- |
| Ledger           | 93%            |
| Network          | 60%            |
| Incentive        | 25%            |
| Stabilitisation  | 15%            |
| Security audit   | 0%             |
| Perf improvement | 1%             |


Ledger
------

Define all the core mechanism of the blockchain:

* transaction, utxos, accounts, multisig
* stake pool, certificates
* cryptographic primitives, consensus primitives
* consensus

The outcome of this phase is that everything is in place for a single node to
expose the blockchain, and developers have the tools necessary to start
developing integration.  Apart from the network component, this also represents
feature complete on the blockchain system.

Network
-------

Define communication between nodes, bootstrapping and allow
node to exchange blocks.

The first goal is to get a somewhat central network perusing the trusted-peers,
then gradually as feature and progress allow move to a fully decentralized
content delivery system with initial point of communication maintained by
trusted-peers.

Incentive
---------

This phase is about definining the rewards, and all the soft mechanisms to
incentivize for good behaviors and penalize bad behaviors.

The initial goal is to start rewarding peers automatically in the network for
their participation. An important side goal is also revisiting and tweaking the
various fees and penalities depending on the data we have gathered so far.

Stabilisation
-------------

This phase is about reviewing and tweaking many of our internals and externals
APIs, with the goals to future proof mechanisms and formats, and doing
systematic internal security review.

Another axis that will be developed here is protection against abuse and monitoring
our resources usages.

It's very important to note that until we have the end of the phase,
anything can change and that security is not guaranteed in any way.

Security Audit
--------------

This phase is about focusing on the last mile, increasing the number of tests,
and specifically internal, cross-team and external audits, and careful
examination of the final code. This phase should remain short, depending on
the results.

Until we finalize this step, use at your own peril.

Scalability & Performance Improvements
--------------------------------------

This phase is a bonus phase, to make sure everything works well and as fast
as possible. There's some low hanging fruits in term of our memory consumption,
and some known (but time consuming) optimisation that we want to complete.
