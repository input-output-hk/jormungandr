# Update semantics

Currently, the update semantics are very simple:

* Update proposal messages can only be made by BFT leaders: they must
  be signed by the secret key of a BFT leader.

* Only BFT leaders can vote for an update proposal: update vote
  messages must be signed by the secret key of a BFT leader.

* Votes are always positive. It is not possible (or necessary) to
  register a vote against a proposal.

* A proposal is accepted if an absolute majority of BFT leaders has
  cast a vote for the proposal. (Note that the update proposal message
  itself is *not* an implied vote for the proposal by the issuing BFT
  leader; that leader needs to separately vote for its own proposal.)
  Thus, with 7 BFT leaders, a proposal is accepted once 4 valid votes
  are on the chain.

* An accepted proposal becomes active at the start of the epoch
  following the slot containg the deciding vote.

* Proposals that change settings other than the consensus version can
  (and must) be applied automatically by any node without a software
  update. A change to the consensus version may require a software
  update, since nodes must be able to process blocks with the new
  version. Since accepted proposals become active almost immediately,
  nodes should only vote in favor of a proposal for a new consensus
  version if they can already support that version.

TODO: what is consensus version exactly? Does it correspond to the
version in the block header? (E.g. we could change the consensus rules
without changing the block format. But if they're different, we
currently don't have a way to identify the intended consensus version
of a block.)

TBD: what to do if multiple versions are accepted at the same time?

TODO: add a way to delay activation of accepted proposals by a number
of epochs. This number could be a setting, or a part of the proposal
(e.g. "this proposal takes effect N epochs after acceptance").

TODO: maybe we can use the update proposal ID as the version? That's a
lot more unique than integers. Also, update proposals can then have a
parent version, so you get a chain of versions. Downside: it makes the
block header a bit bigger.
