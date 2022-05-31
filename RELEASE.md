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

## How to do a release

(As of May 2022)

The release process is automatically started when a matching tag is created (e.g. `v0.15.0`). If you need to do a release, the steps roughly are:
 - create a branch for this release (e.g. `catalyst-fund9`)
 - update the versions of `jormungandr`, `jcli`, and `jormungandr-*` crates to match the version of the tag (i.e. if the tag is `v0.15.0`, the `Cargo.toml` of each crate should have `version = "0.15.0"`)
 - run `cargo build` to update the corresponding version numbers in `Cargo.lock`
 - commit and push those changes to your `origin/<branch-name>`
 - create the tag to start the release process:
   - make sure you've checked out the correct commit
   - create the local tag with `git tag <tag-name>`
   - push the tag to Github with `git push origin --tags`
 - Check the Github Actions tab, if it goes wrong, and you need to try again, use:
   - `git push --delete origin <tag-name>` to delete the tag on Github
   - `git tag --delete <tag-name>` to delete the tag locally
   - `git tag <tag-name>; git push origin --tags` to repeat the tagging process



