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

TBD: what to do if multiple versions are accepted at the same time?

TODO: add a way to delay activation of accepted proposals by a number
of epochs. This number could be a setting, or a part of the proposal
(e.g. "this proposal takes effect N epochs after acceptance").

TODO: maybe we can use the update proposal ID as the version? That's a
lot more unique than integers. Also, update proposals can then have a
parent version, so you get a chain of versions. Downside: it makes the
block header a bit bigger.

TODO: need a way to distinguish between setting changes (which don't
require a software update) and other changes (which do). Currently
UpdateProposal can only do the former.
