# Release process

## Release

release are a specific commit on the 'master' branch

release will happens on master every Wednesday, unless the repository
has not changed since the last release or that master doesn't compile.

A release must have:

* A new version for the node binary and the associated libraries
* Changelog entries in CHANGELOG.md that specify what's new, what got fixed, etc.

A release doesn't mean:

* It will work in various deployment settings
* It's production ready
* It's ready to be deploy everywhere

A release means:

* It passed the internal tests
* It's ready to be tested by more people
* Acceptance tests can be now run
* Integration with other products can be attempted
* Anyone can use it, at their own risk

## Where to get stable releases

Specially as the software matures, the release will get more and more stable,
but the global seal of approval for stability will come from another place.
In parallel of release, we have started thinking of software channels (like chrome)
where we'll have different stability (e.g. stable, beta, experimental).

## Missing the release train

In usual case, release will not wait for a special feature or bugfix. The
release train set off at a specific time, and wait for no one.

In some rare cases, exceptions are likely to be made to deal with certain
scenario (e.g. security updates).

## Versioning

Exact scheme TBD but globally this is following [Semantic Versioning](https://semver.org/)
