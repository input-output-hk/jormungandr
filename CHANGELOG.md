# Change Log

## Unreleased

- Add /v1/account-votes-all endpoint to return the list of proposals a user has voted for
- Remove /v1/account-votes-count endpoint
- Validate server id is the expected one during gRPC handshake
- fix incorrect keys bech32 HRP by always using the ones provided by the library
- update REST API: add new endpoint AccountVotes (`/api/v1/votes/plan/account-votes/{account_id}`)
- Support parallel lanes in spending counters on account outputs. This allows
  submitting transactions that can spend from the same account without
  requiring any particular order between the transactions.
- add new setting proposal expiration into the initial config/genesis
- add feature for changing blockchain config during the network run. Add new certificates `UpdateProposal` and `UpdateVote`, updated `jcli` with these new transactions.
- Add new grpc watch service implementation for external (non-node) clients.
- update explorer, add new GraphQL objects as `UpdateProposal`, `UpdateVote`, `ConfigParam` etc.
- update REST API: add new endpoint account based votes count (`/api/v1/votes/plan/accounts-votes-count`)
- Add jcli option to specify a spending counter lane in a user-friendly way
- Add CORS config params: allowed headers and allowed methods.
- Update REST API `/api/v0/account/{account_id}`, add new field with the token's state info.
- Vote tally now uses the token in the voteplan instead of the native currency.
- Remove `txPendingCnt` metric and related field in `node/stats`
- Rename `txPendingTotalSize` to `mempoolTotalSize`
- Add `mempoolUsageRatio` metric and related field in `node/stats` to track load on the mempool.
- Change `blockContentSizeAvg` to represent information as a percentage of the block max size.
- Bump ed25519-bip32 from 0.4.0 to 0.4.1
- Now the tally is incremental and is always available in the rest API. The
- Add standalone explorer crate.
- Bump clap from 2.34.0 to 3.1.13
- Bump time from 0.3.7 to 0.3.9
- Bump libc from 0.2.117 to 0.2.124
- Bump rand from 0.8.4 to 0.8.5
- Bump os_info from 3.1.0 to 3.3.0
- Bump log from 0.4.14 to 0.4.17
- Bump regex from 1.5.5 to 1.6.0
- Add jcli option to generate and sign EVM mapping certificates.
- Add new Ethereum RPC endpoints for getting block info: eth_getBlockByHash, eth_getBlockByNumber, eth_getBlockTransactionCountByHash, eth_getBlockTransactionCountByNumber, eth_getUncleCountByBlockHash, eth_getUncleCountByBlockNumber, eth_blockNumber
- Add new Ethereum RPC endpoints for transaction handling: eth_sendTransaction, eth_sendRawTransaction, eth_getTransactionByHash, eth_getTransactionByBlockHashAndIndex, eth_getTransactionByBlockNumberAndIndex, eth_getTransactionReceipt, eth_signTransaction, eth_estimateGas, eth_sign, eth_call
- Add new Ethereum RPC endpoints for getting chain info: eth_chainId, eth_syncing, eth_gasPrice, eth_protocolVersion, eth_feeHistory
- Add new Ethereum RPC endpoints for account handling: eth_accounts, eth_getTransactionCount, eth_getBalance, eth_getCode, eth_getStorageAt
- Add new Ethereum RPC filtering endpoints: eth_newFilter, eth_newBlockFilter, eth_newPendingTransactionFilter, eth_uninstallFilter, eth_getFilterChanges, eth_getFilterLogs, eth_getLogs
- Add new Ethereum RPC mining endpoints: eth_mining, eth_coinbase, eth_hashrate, eth_getWork, eth_submitWork, eth_submitHashrate
- Add chain-evm as optional dependency for jcli
- Update gas price and block gas limit for EVM params
- Add new 'evm' REST API endpoints 'address_mapping/jormungandr_address', 'address_mapping/evm_address' for getting info about address mapping. They are optional for the 'evm' feature.
- Add jcli command to merge the results of multiple voteplans with the same proposals.
- Bump rpassword to 6.0.1
- Update implementation for Ethereum RPC transaction endpoints: eth_signTransaction, eth_sign, and eth_call

## Release 0.13.0

**New features:**

- Expiration block date must be set on all incoming fragments.
  Fragments in the mempool that are not included to a block prior to
  the expiration (AKA TTL) block date are rejected.
  A new blockchain parameter configures the maximum number of epochs that
  the expiration date can be ahead when accepted into the mempool.
- Provide node metrics through the Prometheus API.

**Changes:**

- Only a single leader or stake pool is supported in the node configuration.
- Change HRP for Bech32 keys used in the voting process to improve
  discrimination.
- Consistently format block dates as slot.epoch string value in human-readable
  serialization formats. Deserializing from the structured format is supported
  for backward compatibility.
- Voting protocol elliptic curve backend is changed to Ristretto.

**Bug fixes:**

- No reordering for the fragments submitted to the mempool in a batch via the
  REST API.
- Writing of persistent fragment logs was slow, changed to use buffering.

## Release 0.12.0

**New features:**

- Persistent fragment logs, optionally enabled to record all fragments received
  by the node and accepted into the mempool. These logs can be used for
  verification of the blockchain result, forensics, and possibly to compute
  the vote tally from the received fragments off-chain as a backup counting
  method.
- `scripts/bootstrap.py`, a cross-platform Python script to replace tje older
  collection of outdated shell scripts.

**Changes:**

- Updated the Poldercast implementation to use poldercast 1.2
  and reworked quarantine rules to improve network stability.
- The log configuration only deals with a single output backend.
  It's no longer possible to configure multiple log outputs.
- Changed the p2p listening address and port configuration:
  the field name is `listen` and the value format is _addr_`:`_port_.
- The fragment log REST API provides more elaborate information on fragment
  status, including the rejection reason.
- The REST API endpoints for submitting fragments return an error status code
  if the fragments are rejected by the node, rather than being admitted to the
  mempool and propagated across the network.
- Added logging to track REST requests, including possible OpenZipkin/B3
  tracing information from the HTTP headers.

**Bugs fixed:**

- Use voteplan ID as the CRS for private voting protocol to prevent use of
  compromised CRS values.
- Ignore an unworkably small value of `log_max_entries` in the configuration.
  The minimum is `pool_max_entries * n_pools`.

## Releases 0.10.x - 0.11.x

**TODO:** fill in with a summary of changes.

## Release 0.13.0

**New features:**

- Expiration block date must be set on all incoming fragments.
  Fragments in the mempool that are not included to a block prior to
  the expiration (AKA TTL) block date are rejected.
  A new blockchain parameter configures the maximum number of epochs that
  the expiration date can be ahead when accepted into the mempool.
- Provide node metrics through the Prometheus API.

**Changes:**

- Only a single leader or stake pool is supported in the node configuration.
- Change HRP for Bech32 keys used in the voting process to improve
  discrimination.
- Consistently format block dates as slot.epoch string value in human-readable
  serialization formats. Deserializing from the structured format is supported
  for backward compatibility.
- Voting protocol elliptic curve backend is changed to Ristretto.

**Bug fixes:**

- No reordering for the fragments submitted to the mempool in a batch via the
  REST API.
- Writing of persistent fragment logs was slow, changed to use buffering.

## Release 0.12.0

**New features:**

- Persistent fragment logs, optionally enabled to record all fragments received
  by the node and accepted into the mempool. These logs can be used for
  verification of the blockchain result, forensics, and possibly to compute
  the vote tally from the received fragments off-chain as a backup counting
  method.
- `scripts/bootstrap.py`, a cross-platform Python script to replace the older
  collection of outdated shell scripts.

**Changes:**

- Updated the Poldercast implementation to use poldercast 1.2
  and reworked quarantine rules to improve network stability.
- The log configuration only deals with a single output backend.
  It's no longer possible to configure multiple log outputs.
- Changed the p2p listening address and port configuration:
  the field name is `listen` and the value format is _addr_`:`_port_.
- The fragment log REST API provides more elaborate information on fragment
  status, including the rejection reason.
- The REST API endpoints for submitting fragments return an error status code
  if the fragments are rejected by the node, rather than being admitted to the
  mempool and propagated across the network.
- Added logging to track REST requests, including possible OpenZipkin/B3
  tracing information from the HTTP headers.

**Bugs fixed:**

- Use voteplan ID as the CRS for private voting protocol to prevent use of
  compromised CRS values.
- Ignore an unworkably small value of `log_max_entries` in the configuration.
  The minimum is `pool_max_entries * n_pools`.

## Releases 0.10.x - 0.11.x

**TODO:** fill in with a summary of changes.

## [v0.9.3](https://github.com/input-output-hk/jormungandr/tree/v0.9.3) (2020-09-24)

Rolled in recent fixes, testing improvements, and dependency updates for the Catalyst project

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.9.1...v0.9.3)

## [v0.9.1](https://github.com/input-output-hk/jormungandr/tree/v0.9.1) (2020-06-25)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/nightly.20200625...v0.9.1)

**Fixed bugs:**

- 0.9.0 as well as Release 0.9.1-nightly.20200624 not working on Windows [\#2418](https://github.com/input-output-hk/jormungandr/issues/2418)
- explorer: correctly support re-voting [\#2427](https://github.com/input-output-hk/jormungandr/pull/2427)

## [v0.9.0](https://github.com/input-output-hk/jormungandr/tree/v0.9.0) (2020-06-23)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.9.0-rc3...v0.9.0)

## [v0.9.0-rc3](https://github.com/input-output-hk/jormungandr/tree/v0.9.0-rc3) (2020-06-22)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.9.0-rc2...v0.9.0-rc3)

**Implemented enhancements:**

- jcli: voteplan config update [\#2401](https://github.com/input-output-hk/jormungandr/pull/2401)

**Fixed bugs:**

- fix the from address from the trusted peer initial topology [\#2409](https://github.com/input-output-hk/jormungandr/pull/2409)
- Don't skip the node's public id in serde [\#2408](https://github.com/input-output-hk/jormungandr/pull/2408)

**Merged pull requests:**

- \[Tests\] Adversary fragment sender [\#2410](https://github.com/input-output-hk/jormungandr/pull/2410)
- \[Tests\] Public id revert [\#2403](https://github.com/input-output-hk/jormungandr/pull/2403)

## [v0.9.0-rc2](https://github.com/input-output-hk/jormungandr/tree/v0.9.0-rc2) (2020-06-21)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.9.0-rc1...v0.9.0-rc2)

**Implemented enhancements:**

- Tune gRPC connections [\#2402](https://github.com/input-output-hk/jormungandr/pull/2402)
- Protocol update at epoch transition [\#2400](https://github.com/input-output-hk/jormungandr/pull/2400)
- Log received fragments [\#2399](https://github.com/input-output-hk/jormungandr/pull/2399)
- implement `jcli certificate new vote-cast` [\#2398](https://github.com/input-output-hk/jormungandr/pull/2398)
- explorer: get vote plan info by id [\#2360](https://github.com/input-output-hk/jormungandr/pull/2360)

**Fixed bugs:**

- Set up use of "node-id-bin" in server responses [\#2397](https://github.com/input-output-hk/jormungandr/pull/2397)

**Merged pull requests:**

- fix vote tally processing in explorer and add better errors display [\#2406](https://github.com/input-output-hk/jormungandr/pull/2406)
- \[Test\] mesh_disruption test simplify trusted peers net [\#2404](https://github.com/input-output-hk/jormungandr/pull/2404)
- \[Tests\] Network test maintenance [\#2395](https://github.com/input-output-hk/jormungandr/pull/2395)
- Bump async-trait from 0.1.35 to 0.1.36 [\#2394](https://github.com/input-output-hk/jormungandr/pull/2394)

## [v0.9.0-rc1](https://github.com/input-output-hk/jormungandr/tree/v0.9.0-rc1) (2020-06-18)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.19...v0.9.0-rc1)

**Implemented enhancements:**

- factor out `toolchain` from this CI table and add the few outliers with `include`: [\#2310](https://github.com/input-output-hk/jormungandr/issues/2310)
- Update VotePlan & configure it from a file [\#2388](https://github.com/input-output-hk/jormungandr/pull/2388)
- Include extended committees [\#2367](https://github.com/input-output-hk/jormungandr/pull/2367)
- Vote tally and plan status [\#2357](https://github.com/input-output-hk/jormungandr/pull/2357)
- Consistently use optimization flags when building releases [\#2322](https://github.com/input-output-hk/jormungandr/pull/2322)
- settings: allow setting public and listen address from cli args [\#2318](https://github.com/input-output-hk/jormungandr/pull/2318)
- Log incoming gossip on debug level [\#2305](https://github.com/input-output-hk/jormungandr/pull/2305)

**Fixed bugs:**

- Use the same legacy node ID in gossip as in subs [\#2370](https://github.com/input-output-hk/jormungandr/pull/2370)
- Fix error reporting in streaming replies [\#2358](https://github.com/input-output-hk/jormungandr/pull/2358)
- Update chain-network to fix legacy node ID format [\#2348](https://github.com/input-output-hk/jormungandr/pull/2348)
- decode the vote plan with the associated data from transaction [\#2345](https://github.com/input-output-hk/jormungandr/pull/2345)
- Update chain-deps to fix legacy node ID length [\#2331](https://github.com/input-output-hk/jormungandr/pull/2331)

**Closed issues:**

- Legacy node cannot sync with node from master [\#2301](https://github.com/input-output-hk/jormungandr/issues/2301)

**Merged pull requests:**

- Be sure to check out the repo before cargo fetch [\#2392](https://github.com/input-output-hk/jormungandr/pull/2392)
- Bump lru from 0.5.1 to 0.5.2 [\#2391](https://github.com/input-output-hk/jormungandr/pull/2391)
- One release workflow to rule them all [\#2389](https://github.com/input-output-hk/jormungandr/pull/2389)
- \[Tests\] Interactive scenario [\#2387](https://github.com/input-output-hk/jormungandr/pull/2387)
- \[Tests\] Testnet config fix [\#2386](https://github.com/input-output-hk/jormungandr/pull/2386)
- Legacy scenario test fix [\#2384](https://github.com/input-output-hk/jormungandr/pull/2384)
- Bump structopt from 0.3.14 to 0.3.15 [\#2382](https://github.com/input-output-hk/jormungandr/pull/2382)
- Bump tar from 0.4.28 to 0.4.29 [\#2381](https://github.com/input-output-hk/jormungandr/pull/2381)
- Serialize fragments just like we did last summer [\#2380](https://github.com/input-output-hk/jormungandr/pull/2380)
- Save node test logs in directories with other node data [\#2379](https://github.com/input-output-hk/jormungandr/pull/2379)
- \[Tests\] Legacy current node fragment propagation [\#2377](https://github.com/input-output-hk/jormungandr/pull/2377)
- Bump thiserror from 1.0.19 to 1.0.20 [\#2376](https://github.com/input-output-hk/jormungandr/pull/2376)
- Bump base64 from 0.12.1 to 0.12.2 [\#2375](https://github.com/input-output-hk/jormungandr/pull/2375)
- Disable double logging in staging tests [\#2372](https://github.com/input-output-hk/jormungandr/pull/2372)
- \[Test\] fix test_legacy_node_all_fragments test. [\#2371](https://github.com/input-output-hk/jormungandr/pull/2371)
- \[Tests\] shorten resource result string [\#2368](https://github.com/input-output-hk/jormungandr/pull/2368)
- \[Tests\] Add ability to send all types of fragment to FragmentSender [\#2366](https://github.com/input-output-hk/jormungandr/pull/2366)
- Bump serde from 1.0.111 to 1.0.112 [\#2365](https://github.com/input-output-hk/jormungandr/pull/2365)
- Bump custom_debug from 0.4.0 to 0.5.0 [\#2364](https://github.com/input-output-hk/jormungandr/pull/2364)
- Bump zip from 0.5.5 to 0.5.6 [\#2363](https://github.com/input-output-hk/jormungandr/pull/2363)
- Bump pin-project from 0.4.20 to 0.4.22 [\#2362](https://github.com/input-output-hk/jormungandr/pull/2362)
- Bump indicatif from 0.14.0 to 0.15.0 [\#2361](https://github.com/input-output-hk/jormungandr/pull/2361)
- Bump humantime from 2.0.0 to 2.0.1 [\#2356](https://github.com/input-output-hk/jormungandr/pull/2356)
- \[Tests\] resources consumption benchmark for network [\#2355](https://github.com/input-output-hk/jormungandr/pull/2355)
- End binary/library duality in scenario tests [\#2353](https://github.com/input-output-hk/jormungandr/pull/2353)
- Remove giant merged log strings from test output [\#2351](https://github.com/input-output-hk/jormungandr/pull/2351)
- Bump serde_json from 1.0.53 to 1.0.55 [\#2350](https://github.com/input-output-hk/jormungandr/pull/2350)
- Fix file creation vs. check race in tests [\#2349](https://github.com/input-output-hk/jormungandr/pull/2349)
- CircleCI: Limit the doctest threads to 1 [\#2347](https://github.com/input-output-hk/jormungandr/pull/2347)
- Bump serde_yaml from 0.8.12 to 0.8.13 [\#2342](https://github.com/input-output-hk/jormungandr/pull/2342)
- Don't use --all flag with cargo fmt [\#2340](https://github.com/input-output-hk/jormungandr/pull/2340)
- \[Tests\] new test - node enters legacy network [\#2338](https://github.com/input-output-hk/jormungandr/pull/2338)
- introduced log-level cmd arg for private network tests [\#2337](https://github.com/input-output-hk/jormungandr/pull/2337)
- Bump async-trait from 0.1.33 to 0.1.35 [\#2336](https://github.com/input-output-hk/jormungandr/pull/2336)
- \[Tests\] upgrade and downgrade test [\#2335](https://github.com/input-output-hk/jormungandr/pull/2335)
- \[Tests\] change assertion for last stats [\#2334](https://github.com/input-output-hk/jormungandr/pull/2334)
- Reworked CircleCI config [\#2332](https://github.com/input-output-hk/jormungandr/pull/2332)
- Further fix and improve release workflows [\#2329](https://github.com/input-output-hk/jormungandr/pull/2329)
- Bump pin-project from 0.4.19 to 0.4.20 [\#2328](https://github.com/input-output-hk/jormungandr/pull/2328)
- Bump valico from 3.2.0 to 3.4.0 [\#2327](https://github.com/input-output-hk/jormungandr/pull/2327)
- Bump arc-swap from 0.4.6 to 0.4.7 [\#2326](https://github.com/input-output-hk/jormungandr/pull/2326)
- \[Tests\] decrease amount of nodes in real network [\#2324](https://github.com/input-output-hk/jormungandr/pull/2324)
- \[Tests\] Sync module refactoring [\#2323](https://github.com/input-output-hk/jormungandr/pull/2323)
- CI Matrix: Reloaded [\#2319](https://github.com/input-output-hk/jormungandr/pull/2319)
- \[Tests\] Vit block0 tests [\#2317](https://github.com/input-output-hk/jormungandr/pull/2317)
- Bump async-trait from 0.1.32 to 0.1.33 [\#2316](https://github.com/input-output-hk/jormungandr/pull/2316)
- mips and powerpc builds still broken [\#2315](https://github.com/input-output-hk/jormungandr/pull/2315)
- Inject legacy "node-id-bin" metadata [\#2314](https://github.com/input-output-hk/jormungandr/pull/2314)
- Remove unused error enum variants in network [\#2313](https://github.com/input-output-hk/jormungandr/pull/2313)
- \[Test\] Private network test - Fragment dump on send [\#2312](https://github.com/input-output-hk/jormungandr/pull/2312)
- Bump lru from 0.5.0 to 0.5.1 [\#2309](https://github.com/input-output-hk/jormungandr/pull/2309)
- Bump async-trait from 0.1.31 to 0.1.32 [\#2308](https://github.com/input-output-hk/jormungandr/pull/2308)
- ci: add missing parameters to release pipelines [\#2306](https://github.com/input-output-hk/jormungandr/pull/2306)
- Fix indentation in YAML example of logging config [\#2303](https://github.com/input-output-hk/jormungandr/pull/2303)
- Bump yaml-rust from 0.4.3 to 0.4.4 [\#2302](https://github.com/input-output-hk/jormungandr/pull/2302)
- Bump reqwest from 0.10.4 to 0.10.6 [\#2300](https://github.com/input-output-hk/jormungandr/pull/2300)
- Bump serde from 1.0.110 to 1.0.111 [\#2299](https://github.com/input-output-hk/jormungandr/pull/2299)
- Update ring & restore release for PowerPC and MIPS [\#2298](https://github.com/input-output-hk/jormungandr/pull/2298)

## [v0.8.19](https://github.com/input-output-hk/jormungandr/tree/v0.8.19) (2020-05-04)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.18...v0.8.19)

**Implemented enhancements:**

- \[VIT\] REST endpoint to list the committee [\#2070](https://github.com/input-output-hk/jormungandr/issues/2070)
- JCLI - properly expose vote commands [\#2166](https://github.com/input-output-hk/jormungandr/pull/2166)
- DOC - api, update specification. Fixed err/warn [\#2162](https://github.com/input-output-hk/jormungandr/pull/2162)
- add support for script addresses [\#2158](https://github.com/input-output-hk/jormungandr/pull/2158)
- JCli vote plan certificate [\#2157](https://github.com/input-output-hk/jormungandr/pull/2157)
- override package version for nightly builds [\#2146](https://github.com/input-output-hk/jormungandr/pull/2146)
- Expose VotePlans in rest service [\#2135](https://github.com/input-output-hk/jormungandr/pull/2135)
- DOC - cleanup and updates [\#2132](https://github.com/input-output-hk/jormungandr/pull/2132)
- Rest - api, /committees - added [\#2122](https://github.com/input-output-hk/jormungandr/pull/2122)
- Committee block0 [\#2109](https://github.com/input-output-hk/jormungandr/pull/2109)
- add vote plan and cast per certificate fee [\#2107](https://github.com/input-output-hk/jormungandr/pull/2107)
- Add compatibility with the updated new certificate for vote plan [\#2096](https://github.com/input-output-hk/jormungandr/pull/2096)
- Change mutex to std instead of tokio in watchdog intercom [\#2082](https://github.com/input-output-hk/jormungandr/pull/2082)
- doc - api specification maintenance updates [\#2055](https://github.com/input-output-hk/jormungandr/pull/2055)

**Fixed bugs:**

- Build broken on FreeBSD [\#2126](https://github.com/input-output-hk/jormungandr/issues/2126)
- Unregistered pools are still listed in the explorer [\#2074](https://github.com/input-output-hk/jormungandr/issues/2074)
- jormungandr-lib: move sysinfo to dev-dependencies [\#2164](https://github.com/input-output-hk/jormungandr/pull/2164)
- fix the per vote certificate fee configuration in the block0 [\#2110](https://github.com/input-output-hk/jormungandr/pull/2110)
- correctly set tip block in stats counter for non-leader nodes [\#2108](https://github.com/input-output-hk/jormungandr/pull/2108)
- Track retirement in explorer stake pool data [\#2076](https://github.com/input-output-hk/jormungandr/pull/2076)

**Closed issues:**

- Build fails on latest stable Rust 1.43.0 [\#2133](https://github.com/input-output-hk/jormungandr/issues/2133)
- Nightly version numbers not set in binaries [\#2113](https://github.com/input-output-hk/jormungandr/issues/2113)
- REST API Swagger documentation points to the wrong URL for specification file [\#2073](https://github.com/input-output-hk/jormungandr/issues/2073)
- \[VIT\] list active vote plans end points [\#2069](https://github.com/input-output-hk/jormungandr/issues/2069)
- \[VIT\] genesis block0: commitee [\#2067](https://github.com/input-output-hk/jormungandr/issues/2067)
- \[VIT\] jcli update the transaction auth certificate command for vote plan certificates [\#2064](https://github.com/input-output-hk/jormungandr/issues/2064)

**Merged pull requests:**

- JCLI - certificate new vote-plan, fix empty plan [\#2163](https://github.com/input-output-hk/jormungandr/pull/2163)
- GH - actions, update api linter [\#2161](https://github.com/input-output-hk/jormungandr/pull/2161)
- \[Test\] Test maintenance for scenario tests [\#2154](https://github.com/input-output-hk/jormungandr/pull/2154)
- Clippy fixes jormungandr watchdog [\#2144](https://github.com/input-output-hk/jormungandr/pull/2144)
- Chain deps update [\#2143](https://github.com/input-output-hk/jormungandr/pull/2143)
- chain-deps update [\#2142](https://github.com/input-output-hk/jormungandr/pull/2142)
- reduce dependency surface [\#2137](https://github.com/input-output-hk/jormungandr/pull/2137)
- update chain-deps and remove need for LeaderId property in the explorer [\#2134](https://github.com/input-output-hk/jormungandr/pull/2134)
- \[Test\] ignore qa bootstrap network test [\#2131](https://github.com/input-output-hk/jormungandr/pull/2131)
- \[Tests\] genesis decode bijection test [\#2125](https://github.com/input-output-hk/jormungandr/pull/2125)
- \[Test \]Last block update [\#2117](https://github.com/input-output-hk/jormungandr/pull/2117)
- \[Tests\] Retire pool integration test [\#2097](https://github.com/input-output-hk/jormungandr/pull/2097)
- udpate all deps [\#2095](https://github.com/input-output-hk/jormungandr/pull/2095)
- \[Tests\] Settings which allows self reference [\#2087](https://github.com/input-output-hk/jormungandr/pull/2087)
- Bump sysinfo from 0.12.0 to 0.14.1 [\#2085](https://github.com/input-output-hk/jormungandr/pull/2085)
- REST API: replace actix with warp [\#2083](https://github.com/input-output-hk/jormungandr/pull/2083)
- \[Tests\] Add assertion for pool_id from retirement cert [\#2081](https://github.com/input-output-hk/jormungandr/pull/2081)
- \[Tests\] Quarantine White-list tests [\#2080](https://github.com/input-output-hk/jormungandr/pull/2080)
- separate REST Context and methods implementations from Actix [\#2078](https://github.com/input-output-hk/jormungandr/pull/2078)
- doc - api v0, cleaning [\#2077](https://github.com/input-output-hk/jormungandr/pull/2077)
- \[Tests\] updated openapi.yaml location. updated error messages [\#2071](https://github.com/input-output-hk/jormungandr/pull/2071)
- \[Tests\] Implement scenario tests in integration tests [\#2062](https://github.com/input-output-hk/jormungandr/pull/2062)
- \[Test\] private scenario tests connection limits test cases [\#2047](https://github.com/input-output-hk/jormungandr/pull/2047)
- Fix version not working for crosscompiled targets [\#2046](https://github.com/input-output-hk/jormungandr/pull/2046)

## [v0.8.18](https://github.com/input-output-hk/jormungandr/tree/v0.8.18) (2020-04-10)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/nightly...v0.8.18)

**Implemented enhancements:**

- Nodes Compatibility test [\#1997](https://github.com/input-output-hk/jormungandr/issues/1997)
- Implemented derive macro for IntercomMsg [\#2034](https://github.com/input-output-hk/jormungandr/pull/2034)

**Fixed bugs:**

- Node stats shows wrong value for peer_connected_cnt [\#1977](https://github.com/input-output-hk/jormungandr/issues/1977)
- Node created a strange block on slot 97.060 on March 19 [\#1942](https://github.com/input-output-hk/jormungandr/issues/1942)
- always make chain selection in process_new_ref [\#2052](https://github.com/input-output-hk/jormungandr/pull/2052)

**Merged pull requests:**

- \[Tests\] move multi node legacy tests to stable build [\#2044](https://github.com/input-output-hk/jormungandr/pull/2044)
- \[Tests\] Duplicated nodes id scenario test [\#2043](https://github.com/input-output-hk/jormungandr/pull/2043)
- Nightly release with version and date [\#2036](https://github.com/input-output-hk/jormungandr/pull/2036)
- Nightly build recreate fix [\#2021](https://github.com/input-output-hk/jormungandr/pull/2021)
- \[Tests\] Jts legacy test [\#2010](https://github.com/input-output-hk/jormungandr/pull/2010)

## [v0.8.17](https://github.com/input-output-hk/jormungandr/tree/v0.8.17) (2020-04-01)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.16...v0.8.17)

**Implemented enhancements:**

- Network whitelist: prevent specific addresses \(pool id\) to be quarantined forever [\#1973](https://github.com/input-output-hk/jormungandr/issues/1973)
- Preferred list for poldercast layer [\#1990](https://github.com/input-output-hk/jormungandr/pull/1990)
- Added whitelisting to Policy [\#1982](https://github.com/input-output-hk/jormungandr/pull/1982)
- Access certain REST API methods before bootstrap is over [\#1981](https://github.com/input-output-hk/jormungandr/pull/1981)

**Fixed bugs:**

- When starting gossiping, network does not check for already connected node [\#1946](https://github.com/input-output-hk/jormungandr/issues/1946)
- get_stats_counter: do not return errors when missing state fields [\#2000](https://github.com/input-output-hk/jormungandr/pull/2000)
- Fix node stats peer_connected_cnt [\#1980](https://github.com/input-output-hk/jormungandr/pull/1980)
- Initiate gossips fix 1946 [\#1970](https://github.com/input-output-hk/jormungandr/pull/1970)

**Closed issues:**

- API feature request - /api/v0/leaders/logs/{leader_id} [\#1983](https://github.com/input-output-hk/jormungandr/issues/1983)

**Merged pull requests:**

- Bump assert_cmd from 1.0.0 to 1.0.1 [\#1996](https://github.com/input-output-hk/jormungandr/pull/1996)
- Bump proc-macro2 from 1.0.9 to 1.0.10 [\#1995](https://github.com/input-output-hk/jormungandr/pull/1995)
- use the exact version of rustls in jormungandr [\#1994](https://github.com/input-output-hk/jormungandr/pull/1994)
- \[Tests\] P2p stats test [\#1993](https://github.com/input-output-hk/jormungandr/pull/1993)
- \[Tests\] implemented network stats geters. Starter improvements [\#1992](https://github.com/input-output-hk/jormungandr/pull/1992)
- Bump async-trait from 0.1.24 to 0.1.27 [\#1989](https://github.com/input-output-hk/jormungandr/pull/1989)
- Bump assert_fs from 0.13.1 to 1.0.0 [\#1988](https://github.com/input-output-hk/jormungandr/pull/1988)
- Bump serde_json from 1.0.48 to 1.0.50 [\#1987](https://github.com/input-output-hk/jormungandr/pull/1987)
- Bump thiserror from 1.0.11 to 1.0.14 [\#1986](https://github.com/input-output-hk/jormungandr/pull/1986)
- update all crates to use the same version of reqwest [\#1976](https://github.com/input-output-hk/jormungandr/pull/1976)
- Bump assert_cmd from 0.12.0 to 1.0.0 [\#1975](https://github.com/input-output-hk/jormungandr/pull/1975)
- use OpenSSL for reqwests 0.10 on Android [\#1971](https://github.com/input-output-hk/jormungandr/pull/1971)
- Bump proc-macro-error from 0.4.11 to 1.0.0 [\#1967](https://github.com/input-output-hk/jormungandr/pull/1967)
- Bump syn from 1.0.16 to 1.0.17 [\#1965](https://github.com/input-output-hk/jormungandr/pull/1965)
- Bump sysinfo from 0.11.7 to 0.12.0 [\#1964](https://github.com/input-output-hk/jormungandr/pull/1964)
- Bump ed25519-bip32 from 0.3.0 to 0.3.1 [\#1963](https://github.com/input-output-hk/jormungandr/pull/1963)
- Bump serde from 1.0.104 to 1.0.105 [\#1937](https://github.com/input-output-hk/jormungandr/pull/1937)

## [v0.8.16](https://github.com/input-output-hk/jormungandr/tree/v0.8.16) (2020-03-26)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.15...v0.8.16)

**Implemented enhancements:**

- release: create optimised x86-64 binaries release [\#1924](https://github.com/input-output-hk/jormungandr/issues/1924)
- jcli: certificate get-stake-pool-id - make valid also for retirement [\#1950](https://github.com/input-output-hk/jormungandr/pull/1950)
- jcli: /api/v0/rewards - \(history,epoch\) - exposed [\#1913](https://github.com/input-output-hk/jormungandr/pull/1913)

**Fixed bugs:**

- Remove asserts when searching for epoch distribution [\#1945](https://github.com/input-output-hk/jormungandr/issues/1945)

**Closed issues:**

- Since upgrading to 0.8.14-9ca427ef+, node gets stuck. [\#1927](https://github.com/input-output-hk/jormungandr/issues/1927)

**Merged pull requests:**

- compile the tests and the doc in separate jobs [\#1968](https://github.com/input-output-hk/jormungandr/pull/1968)
- switch to rustls from openssl [\#1961](https://github.com/input-output-hk/jormungandr/pull/1961)
- rename max_client_connections [\#1960](https://github.com/input-output-hk/jormungandr/pull/1960)
- Bump regex from 1.3.4 to 1.3.6 [\#1958](https://github.com/input-output-hk/jormungandr/pull/1958)
- Add simd optimization for x86-64 targets on release [\#1955](https://github.com/input-output-hk/jormungandr/pull/1955)
- \[Tests\] Move failing tests from private network tests to unstable build [\#1953](https://github.com/input-output-hk/jormungandr/pull/1953)
- \[Tests\] Fix network builder in real network test case [\#1952](https://github.com/input-output-hk/jormungandr/pull/1952)
- \[Private network tests\] progress bar mode which only prints scenario results [\#1951](https://github.com/input-output-hk/jormungandr/pull/1951)
- Remove asserts when searching for epoch distribution [\#1947](https://github.com/input-output-hk/jormungandr/pull/1947)
- Bump structopt from 0.3.11 to 0.3.12 [\#1939](https://github.com/input-output-hk/jormungandr/pull/1939)
- Bump slog-async from 2.4.0 to 2.5.0 [\#1938](https://github.com/input-output-hk/jormungandr/pull/1938)
- Bump arc-swap from 0.4.4 to 0.4.5 [\#1929](https://github.com/input-output-hk/jormungandr/pull/1929)
- Bump libc from 0.2.67 to 0.2.68 [\#1928](https://github.com/input-output-hk/jormungandr/pull/1928)

## [v0.8.15](https://github.com/input-output-hk/jormungandr/tree/v0.8.15) (2020-03-18)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.14...v0.8.15)

**Implemented enhancements:**

- Added peer connected count to stats [\#1918](https://github.com/input-output-hk/jormungandr/pull/1918)
- p2p quarantine policy update and vicinity randomnisation [\#1916](https://github.com/input-output-hk/jormungandr/pull/1916)
- jcli: /api/v0/stake/{epoch} - exposed [\#1910](https://github.com/input-output-hk/jormungandr/pull/1910)

**Closed issues:**

- One Stats to rule them all [\#1919](https://github.com/input-output-hk/jormungandr/issues/1919)
- Document the steps for jcli pool retirement [\#1906](https://github.com/input-output-hk/jormungandr/issues/1906)

**Merged pull requests:**

- \[Tests\] Private network multiple trust fix [\#1926](https://github.com/input-output-hk/jormungandr/pull/1926)
- Rest: node stats - use interface dto [\#1923](https://github.com/input-output-hk/jormungandr/pull/1923)
- \[Tests\] System resources monitoring for node [\#1922](https://github.com/input-output-hk/jormungandr/pull/1922)
- Changed default value of max_client_connections [\#1917](https://github.com/input-output-hk/jormungandr/pull/1917)
- \[Test\] Reward history test case [\#1914](https://github.com/input-output-hk/jormungandr/pull/1914)
- \[Docs\] jcli retirement docs [\#1909](https://github.com/input-output-hk/jormungandr/pull/1909)
- blockchain: convert internals to futures 0.3 [\#1908](https://github.com/input-output-hk/jormungandr/pull/1908)
- move different modules to new tokio runtime [\#1907](https://github.com/input-output-hk/jormungandr/pull/1907)
- \[Tests\] test case for leadership log parent hash [\#1905](https://github.com/input-output-hk/jormungandr/pull/1905)
- \[Tests\]Explorer soak test [\#1902](https://github.com/input-output-hk/jormungandr/pull/1902)

## [v0.8.14](https://github.com/input-output-hk/jormungandr/tree/v0.8.14) (2020-03-12)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.13...v0.8.14)

**Implemented enhancements:**

- put parent hash for created block in leaders logs [\#1881](https://github.com/input-output-hk/jormungandr/issues/1881)
- access random stake distribution in time [\#1901](https://github.com/input-output-hk/jormungandr/pull/1901)
- Reward history [\#1886](https://github.com/input-output-hk/jormungandr/pull/1886)
- add parent hash for created block in leadership logs [\#1883](https://github.com/input-output-hk/jormungandr/pull/1883)
- add Stake Pool retirement command in JCLI [\#1877](https://github.com/input-output-hk/jormungandr/pull/1877)
- Blockchain tip tracking in stats_counter [\#1809](https://github.com/input-output-hk/jormungandr/pull/1809)

**Fixed bugs:**

- rust compile issues - rustc 1.41.1 \(f3e1a954d 2020-02-24\) [\#1879](https://github.com/input-output-hk/jormungandr/issues/1879)
- aarch64-unknown-linux-gnu is missing in the release [\#1856](https://github.com/input-output-hk/jormungandr/issues/1856)
- stuck_notifier: get current time before tip_date [\#1867](https://github.com/input-output-hk/jormungandr/pull/1867)

**Closed issues:**

- Please investigate a potential vector to a Sybil attack [\#1899](https://github.com/input-output-hk/jormungandr/issues/1899)
- A Single Node Generated an Adversarial Fork [\#1890](https://github.com/input-output-hk/jormungandr/issues/1890)
- v0.8.9 I get the error report of node stuck, but the node running well [\#1824](https://github.com/input-output-hk/jormungandr/issues/1824)

**Merged pull requests:**

- Use custom CircleCI image [\#1904](https://github.com/input-output-hk/jormungandr/pull/1904)
- \[Tests\] jcli retirement test case [\#1903](https://github.com/input-output-hk/jormungandr/pull/1903)
- Bump rand_chacha from 0.2.1 to 0.2.2 [\#1900](https://github.com/input-output-hk/jormungandr/pull/1900)
- \[Tests\] Change 'relay' test case tag - removed unstable [\#1898](https://github.com/input-output-hk/jormungandr/pull/1898)
- Bump base64 from 0.11.0 to 0.12.0 [\#1895](https://github.com/input-output-hk/jormungandr/pull/1895)
- Bump chrono from 0.4.10 to 0.4.11 [\#1894](https://github.com/input-output-hk/jormungandr/pull/1894)
- Bump console from 0.9.2 to 0.10.0 [\#1893](https://github.com/input-output-hk/jormungandr/pull/1893)
- \[Tests\] Stabilize disruption tests [\#1892](https://github.com/input-output-hk/jormungandr/pull/1892)
- Bump tracing-subscriber from 0.2.2 to 0.2.3 [\#1888](https://github.com/input-output-hk/jormungandr/pull/1888)
- update the documentation and make sure we use the `--locked` `Cargo.lock` [\#1880](https://github.com/input-output-hk/jormungandr/pull/1880)
- Bump tokio-compat from 0.1.4 to 0.1.5 [\#1878](https://github.com/input-output-hk/jormungandr/pull/1878)
- \[Tests\] move wallet transaction logic to libs [\#1876](https://github.com/input-output-hk/jormungandr/pull/1876)
- Bump proc-macro-error from 0.4.9 to 0.4.11 [\#1875](https://github.com/input-output-hk/jormungandr/pull/1875)
- Bump structopt from 0.3.9 to 0.3.11 [\#1874](https://github.com/input-output-hk/jormungandr/pull/1874)
- \[Private network tests\] Leader promotion [\#1871](https://github.com/input-output-hk/jormungandr/pull/1871)
- Bump tokio2 deps to 0.2.12 [\#1870](https://github.com/input-output-hk/jormungandr/pull/1870)
- Bump tracing-subscriber from 0.2.1 to 0.2.2 [\#1869](https://github.com/input-output-hk/jormungandr/pull/1869)
- Warnings cleaning [\#1866](https://github.com/input-output-hk/jormungandr/pull/1866)
- Bump tracing-futures from 0.2.2 to 0.2.3 [\#1864](https://github.com/input-output-hk/jormungandr/pull/1864)
- Bump tracing from 0.1.12 to 0.1.13 [\#1863](https://github.com/input-output-hk/jormungandr/pull/1863)
- Cargo lock update [\#1861](https://github.com/input-output-hk/jormungandr/pull/1861)
- GitHub actions mdbook build and deploy [\#1860](https://github.com/input-output-hk/jormungandr/pull/1860)
- GitHub actions cross compiling fix [\#1859](https://github.com/input-output-hk/jormungandr/pull/1859)
- storage: make use of busy timeouts and remove lock [\#1853](https://github.com/input-output-hk/jormungandr/pull/1853)
- \[Tests\] Scenario tests disruption [\#1850](https://github.com/input-output-hk/jormungandr/pull/1850)
- Update build documentation in README [\#1846](https://github.com/input-output-hk/jormungandr/pull/1846)
- Structopt to 0.3 [\#1845](https://github.com/input-output-hk/jormungandr/pull/1845)
- allow getting blocks by chain height [\#1797](https://github.com/input-output-hk/jormungandr/pull/1797)

## [v0.8.13](https://github.com/input-output-hk/jormungandr/tree/v0.8.13) (2020-02-26)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.12...v0.8.13)

**Implemented enhancements:**

- Avoid inconsistency between block store and state [\#1852](https://github.com/input-output-hk/jormungandr/pull/1852)

**Fixed bugs:**

- Promoting Leader is broken in 0.8.12 [\#1857](https://github.com/input-output-hk/jormungandr/issues/1857)
- huge simplification of the code for the leader enclave holder [\#1858](https://github.com/input-output-hk/jormungandr/pull/1858)

**Merged pull requests:**

- Publish docs with github actions [\#1854](https://github.com/input-output-hk/jormungandr/pull/1854)

## [v0.8.12](https://github.com/input-output-hk/jormungandr/tree/v0.8.12) (2020-02-25)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.11...v0.8.12)

**Breaking changes:**

- Use LRU instead of DelayQueue in fragment module [\#1828](https://github.com/input-output-hk/jormungandr/pull/1828)
- remove log_ttl for the leadership and use a LruCache instead [\#1825](https://github.com/input-output-hk/jormungandr/pull/1825)

**Implemented enhancements:**

- netboot is too quiet [\#1819](https://github.com/input-output-hk/jormungandr/issues/1819)
- speedup loading from storage [\#1851](https://github.com/input-output-hk/jormungandr/pull/1851)
- Time limits on client connections [\#1836](https://github.com/input-output-hk/jormungandr/pull/1836)
- Netboot improvements [\#1822](https://github.com/input-output-hk/jormungandr/pull/1822)
- storage: Optimize PumpedStream [\#1817](https://github.com/input-output-hk/jormungandr/pull/1817)
- More async [\#1814](https://github.com/input-output-hk/jormungandr/pull/1814)

**Fixed bugs:**

- grpc connect / shortly after seems to get stuck permanently [\#1829](https://github.com/input-output-hk/jormungandr/issues/1829)
- Enclave avoid duplicated Leaders [\#1760](https://github.com/input-output-hk/jormungandr/pull/1760)

**Closed issues:**

- Super slow loading from storage 0.8.10-0.8.11 [\#1810](https://github.com/input-output-hk/jormungandr/issues/1810)

**Merged pull requests:**

- Bump syn from 1.0.15 to 1.0.16 [\#1848](https://github.com/input-output-hk/jormungandr/pull/1848)
- Bump proc-macro2 from 1.0.8 to 1.0.9 [\#1847](https://github.com/input-output-hk/jormungandr/pull/1847)
- Restore connect error message formatting [\#1844](https://github.com/input-output-hk/jormungandr/pull/1844)
- Bump syn from 1.0.14 to 1.0.15 [\#1843](https://github.com/input-output-hk/jormungandr/pull/1843)
- fix build warning in stuck notifier [\#1833](https://github.com/input-output-hk/jormungandr/pull/1833)
- Remove delay queue from ref cache [\#1826](https://github.com/input-output-hk/jormungandr/pull/1826)
- Update the stuck notifier to work in standard future [\#1823](https://github.com/input-output-hk/jormungandr/pull/1823)
- Bump libc from 0.2.66 to 0.2.67 [\#1821](https://github.com/input-output-hk/jormungandr/pull/1821)
- Bump error-chain from 0.12.1 to 0.12.2 [\#1820](https://github.com/input-output-hk/jormungandr/pull/1820)
- \[Test\] Benchmark api for tests [\#1805](https://github.com/input-output-hk/jormungandr/pull/1805)

## [v0.8.11](https://github.com/input-output-hk/jormungandr/tree/v0.8.11) (2020-02-20)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.10-2...v0.8.11)

**Implemented enhancements:**

- Improve fetch of block0 using easy HTTP services [\#1799](https://github.com/input-output-hk/jormungandr/pull/1799)
- Use peers retrieved during bootstrap [\#1794](https://github.com/input-output-hk/jormungandr/pull/1794)

**Closed issues:**

- 0.8.10 Bootstrap elapsed time X2 [\#1795](https://github.com/input-output-hk/jormungandr/issues/1795)
- jormungandr-v0.8.10-2-x86_64-unknown-linux-gnu.tar.gz - error while loading shared libraries: libssl.so.1.1 [\#1791](https://github.com/input-output-hk/jormungandr/issues/1791)
- aarch64-unknown-linux-gnu is missing in 0.8.10 release [\#1786](https://github.com/input-output-hk/jormungandr/issues/1786)
- v0.8.10 jormungandr-v0.8.10-x86_64-unknown-linux-musl.tar.gz not working [\#1785](https://github.com/input-output-hk/jormungandr/issues/1785)

**Merged pull requests:**

- Bump chain-deps from `10993cc` to `6fa2921` [\#1807](https://github.com/input-output-hk/jormungandr/pull/1807)
- Bump hex from 0.4.1 to 0.4.2 [\#1806](https://github.com/input-output-hk/jormungandr/pull/1806)
- Lockless fragment process [\#1804](https://github.com/input-output-hk/jormungandr/pull/1804)
- Bump chain-deps from `076c586` to `10993cc` [\#1801](https://github.com/input-output-hk/jormungandr/pull/1801)
- poldercast update to 0.11.3 [\#1800](https://github.com/input-output-hk/jormungandr/pull/1800)
- Bump thiserror from 1.0.10 to 1.0.11 [\#1798](https://github.com/input-output-hk/jormungandr/pull/1798)
- Bump serde_json from 1.0.47 to 1.0.48 [\#1773](https://github.com/input-output-hk/jormungandr/pull/1773)

## [v0.8.10](https://github.com/input-output-hk/jormungandr/tree/v0.8.10) (2020-02-13)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.9...v0.8.10)

**Breaking changes:**

- integrate pool-upgrade from chain-libs [\#1766](https://github.com/input-output-hk/jormungandr/pull/1766)

**Implemented enhancements:**

- Don't hold the fragment pool lock for too long [\#1779](https://github.com/input-output-hk/jormungandr/pull/1779)
- Add \(somewhat hardcoded peers queries [\#1778](https://github.com/input-output-hk/jormungandr/pull/1778)
- Integrate lock-free Multiverse in mockchain [\#1749](https://github.com/input-output-hk/jormungandr/pull/1749)
- improve logging of the stuck notifier [\#1743](https://github.com/input-output-hk/jormungandr/pull/1743)
- Separate limit for client connections [\#1696](https://github.com/input-output-hk/jormungandr/pull/1696)
- Asynchronous storage query execution and connection pool [\#1655](https://github.com/input-output-hk/jormungandr/pull/1655)

**Fixed bugs:**

- rest - /api/v0/stake_pool/{pool_id} - total_stake wrong value [\#1772](https://github.com/input-output-hk/jormungandr/issues/1772)
- Don't block in the logs and the pools [\#1780](https://github.com/input-output-hk/jormungandr/pull/1780)
- report the stake for the selected pool instead of the whole system stake [\#1776](https://github.com/input-output-hk/jormungandr/pull/1776)

**Security fixes:**

- Update ledger ed25519 signing to the new signing closure [\#1703](https://github.com/input-output-hk/jormungandr/pull/1703)

**Closed issues:**

- test jormungandr::mempool::test_log_ttl is failing randomly [\#1774](https://github.com/input-output-hk/jormungandr/issues/1774)
- Leader Logs Missing Blocks that were previously there - node doesn't create them either [\#1771](https://github.com/input-output-hk/jormungandr/issues/1771)
- Consider using libp2p [\#1769](https://github.com/input-output-hk/jormungandr/issues/1769)
- `jcli rest v0 stake-pool get` and `jcli rest v0 stake get` are returning different stake delegation per pool [\#1747](https://github.com/input-output-hk/jormungandr/issues/1747)
- Inconsistent reward dump [\#1722](https://github.com/input-output-hk/jormungandr/issues/1722)
- IOHK node rejecting bootstrap pull request . Not able to bootstrap from trusted peer of IOHK [\#1692](https://github.com/input-output-hk/jormungandr/issues/1692)
- Jormungandr 0.8.7 node does not create blocks on Windows [\#1659](https://github.com/input-output-hk/jormungandr/issues/1659)

**Merged pull requests:**

- fix sqlite failute in block validation [\#1784](https://github.com/input-output-hk/jormungandr/pull/1784)
- use the View instead of the All [\#1782](https://github.com/input-output-hk/jormungandr/pull/1782)
- \[Tests\] remove duplicated tests after functional test conversion [\#1768](https://github.com/input-output-hk/jormungandr/pull/1768)
- upgrade bech32 to 0.7 [\#1764](https://github.com/input-output-hk/jormungandr/pull/1764)
- \[Tests\] Converted self node perf test from assertion to measurement based [\#1763](https://github.com/input-output-hk/jormungandr/pull/1763)
- Bump async-trait from 0.1.22 to 0.1.24 [\#1759](https://github.com/input-output-hk/jormungandr/pull/1759)
- Bump serde_json from 1.0.46 to 1.0.47 [\#1757](https://github.com/input-output-hk/jormungandr/pull/1757)
- Bump hex from 0.4.0 to 0.4.1 [\#1756](https://github.com/input-output-hk/jormungandr/pull/1756)
- Input process async [\#1754](https://github.com/input-output-hk/jormungandr/pull/1754)
- \[Test\] Quarantine update in private network tests [\#1753](https://github.com/input-output-hk/jormungandr/pull/1753)
- update chain-deps with merged sqlite connections [\#1752](https://github.com/input-output-hk/jormungandr/pull/1752)
- fragment: convert to std::future [\#1750](https://github.com/input-output-hk/jormungandr/pull/1750)
- Release pipeline for GitHub actions [\#1748](https://github.com/input-output-hk/jormungandr/pull/1748)
- Bump valico from 3.1.0 to 3.2.0 [\#1746](https://github.com/input-output-hk/jormungandr/pull/1746)
- convert storage bootstrap to std::futures [\#1745](https://github.com/input-output-hk/jormungandr/pull/1745)
- Add std::future compliant version of spawn and run_periodic [\#1744](https://github.com/input-output-hk/jormungandr/pull/1744)
- chain-deps updates [\#1742](https://github.com/input-output-hk/jormungandr/pull/1742)
- node stats update [\#1740](https://github.com/input-output-hk/jormungandr/pull/1740)
- Do not truncate the list of peers for propagation [\#1738](https://github.com/input-output-hk/jormungandr/pull/1738)
- \[Tests\] updated node stats with total_peer_cnt and node_id fields [\#1733](https://github.com/input-output-hk/jormungandr/pull/1733)
- Lock down the AppVeyor build image [\#1732](https://github.com/input-output-hk/jormungandr/pull/1732)
- \[Tests\] Move remove address duplication in integration tests [\#1731](https://github.com/input-output-hk/jormungandr/pull/1731)
- Bump nix from 0.15.0 to 0.17.0 [\#1730](https://github.com/input-output-hk/jormungandr/pull/1730)
- Bump base64 from 0.10.1 to 0.11.0 [\#1727](https://github.com/input-output-hk/jormungandr/pull/1727)
- Bump humantime from 1.3.0 to 2.0.0 [\#1726](https://github.com/input-output-hk/jormungandr/pull/1726)
- Bump tokio-threadpool from 0.1.17 to 0.1.18 [\#1723](https://github.com/input-output-hk/jormungandr/pull/1723)
- Bump console from 0.7.7 to 0.9.2 [\#1721](https://github.com/input-output-hk/jormungandr/pull/1721)
- Added node ID to stats output [\#1720](https://github.com/input-output-hk/jormungandr/pull/1720)
- IPv6 dafault to IPv4 methods in gossip is_global [\#1717](https://github.com/input-output-hk/jormungandr/pull/1717)
- use tokio-compat runtime instead of tokio 0.1 [\#1715](https://github.com/input-output-hk/jormungandr/pull/1715)
- Bump hex from 0.3.2 to 0.4.0 [\#1712](https://github.com/input-output-hk/jormungandr/pull/1712)
- Bump indicatif from 0.11.0 to 0.14.0 [\#1711](https://github.com/input-output-hk/jormungandr/pull/1711)
- Bump thiserror from 1.0.9 to 1.0.10 [\#1710](https://github.com/input-output-hk/jormungandr/pull/1710)
- Bump slog-async from 2.3.0 to 2.4.0 [\#1707](https://github.com/input-output-hk/jormungandr/pull/1707)
- Bump serde_json from 1.0.45 to 1.0.46 [\#1706](https://github.com/input-output-hk/jormungandr/pull/1706)
- Bump juniper from 0.13.1 to 0.14.2 [\#1705](https://github.com/input-output-hk/jormungandr/pull/1705)
- Bump assert_cmd from 0.11.1 to 0.12.0 [\#1704](https://github.com/input-output-hk/jormungandr/pull/1704)
- Convert leadership module to tokio 0.2 and futures 0.3 [\#1700](https://github.com/input-output-hk/jormungandr/pull/1700)
- Refactor REST API to remove sync locking and make it more isolated [\#1698](https://github.com/input-output-hk/jormungandr/pull/1698)
- Update poldercast library and use performant node count [\#1691](https://github.com/input-output-hk/jormungandr/pull/1691)
- Docs - update with latest changes [\#1684](https://github.com/input-output-hk/jormungandr/pull/1684)
- \[Tests\] Private network tests - listen/public address split [\#1683](https://github.com/input-output-hk/jormungandr/pull/1683)
- \[Tests\] another timeout extend for testnet [\#1668](https://github.com/input-output-hk/jormungandr/pull/1668)

## [v0.8.9](https://github.com/input-output-hk/jormungandr/tree/v0.8.9) (2020-01-30)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.8...v0.8.9)

**Fixed bugs:**

- Tokio Thread Panicked: Validated Block Must Be Unique [\#1677](https://github.com/input-output-hk/jormungandr/issues/1677)

**Closed issues:**

- node crashed shortly after upgrade 0.8.8 [\#1679](https://github.com/input-output-hk/jormungandr/issues/1679)

**Merged pull requests:**

- Use checkpoints when bootstrapping [\#1682](https://github.com/input-output-hk/jormungandr/pull/1682)

## [v0.8.8](https://github.com/input-output-hk/jormungandr/tree/v0.8.8) (2020-01-30)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.7...v0.8.8)

**Implemented enhancements:**

- Increase selection information in the existing chain-selection/application [\#1670](https://github.com/input-output-hk/jormungandr/issues/1670)
- bootstrap with large number of blocks looked block [\#1669](https://github.com/input-output-hk/jormungandr/issues/1669)
- Update chain-deps [\#1674](https://github.com/input-output-hk/jormungandr/pull/1674)
- Add/Improve some early logging and network related choices [\#1671](https://github.com/input-output-hk/jormungandr/pull/1671)
- Fixes bootstrapping from trusted peers falling through when node is n… [\#1646](https://github.com/input-output-hk/jormungandr/pull/1646)

**Fixed bugs:**

- DatabaseLocked error when using in-memory database [\#1638](https://github.com/input-output-hk/jormungandr/issues/1638)
- Wrong parent hash selection due to low number of block announcements [\#1596](https://github.com/input-output-hk/jormungandr/issues/1596)
- integrate fix for DatabaseLocked error [\#1641](https://github.com/input-output-hk/jormungandr/pull/1641)

**Closed issues:**

- As a small stake pool operator \(and hopefully as an “anyone else”\), I would like to have incentives baked into a Cardano Constitution \(and the Ouroboros protocol\), so that we can assure the ongoing decentralization of Cardano ecosystem. [\#1657](https://github.com/input-output-hk/jormungandr/issues/1657)
- Competitive fork slot and timestamps do not match [\#1651](https://github.com/input-output-hk/jormungandr/issues/1651)
- Compiled v0.8.7 jcli binary doesn't execute MacOS Catalina [\#1644](https://github.com/input-output-hk/jormungandr/issues/1644)
- Tokio-runtime PoisonError Panic, v0.8.6 [\#1643](https://github.com/input-output-hk/jormungandr/issues/1643)
- REST API STOPS RESPONDING WHILE JORMUNGANDR CONTINUES TO RUN [\#1642](https://github.com/input-output-hk/jormungandr/issues/1642)
- Remote node not yet fully connected should not be picked for fetching block, otherwise it fails - block fetch from xxxx failed: PropagateError { kind: NotSubscribed,... [\#1630](https://github.com/input-output-hk/jormungandr/issues/1630)
- What is the Genesis Block Hash for 0.8.6? [\#1629](https://github.com/input-output-hk/jormungandr/issues/1629)
- Different leader schedule showing on 0.8.7 than 0.8.6 [\#1624](https://github.com/input-output-hk/jormungandr/issues/1624)

**Merged pull requests:**

- Poldercast update 0.11.1 [\#1672](https://github.com/input-output-hk/jormungandr/pull/1672)
- \[Tests\] Changed public and listen port to be different [\#1667](https://github.com/input-output-hk/jormungandr/pull/1667)
- \[Tests\] Print logs to console on error [\#1666](https://github.com/input-output-hk/jormungandr/pull/1666)
- \[Tests\] Private network tests improvements [\#1665](https://github.com/input-output-hk/jormungandr/pull/1665)
- Block processing fixes [\#1661](https://github.com/input-output-hk/jormungandr/pull/1661)
- Display the cause of ListenError [\#1660](https://github.com/input-output-hk/jormungandr/pull/1660)
- Go back to local runtimes again [\#1658](https://github.com/input-output-hk/jormungandr/pull/1658)
- Trace spawned futures [\#1656](https://github.com/input-output-hk/jormungandr/pull/1656)
- \[Tests\] update quarantine stats from string to u32 [\#1650](https://github.com/input-output-hk/jormungandr/pull/1650)
- \[Tests\] wait for block sync [\#1649](https://github.com/input-output-hk/jormungandr/pull/1649)
- \[Tests\] Performance block sync test [\#1648](https://github.com/input-output-hk/jormungandr/pull/1648)
- Changed peers counts from strings to int [\#1640](https://github.com/input-output-hk/jormungandr/pull/1640)
- Bypass peers that are not connected for fetching blocks [\#1633](https://github.com/input-output-hk/jormungandr/pull/1633)
- Disable the audit job in CircleCI [\#1632](https://github.com/input-output-hk/jormungandr/pull/1632)
- \[Tests\] dumps log on transaction not in block when timeout is reached [\#1631](https://github.com/input-output-hk/jormungandr/pull/1631)
- \[Tests\] more logs for failed 'transaction is block' assertion [\#1628](https://github.com/input-output-hk/jormungandr/pull/1628)
- \[Tests\] Updated NodeStats with new fields [\#1627](https://github.com/input-output-hk/jormungandr/pull/1627)
- \[Tests\] Private network test maintenance [\#1621](https://github.com/input-output-hk/jormungandr/pull/1621)
- \[Tests\] Move config structs to jormungandr lib [\#1620](https://github.com/input-output-hk/jormungandr/pull/1620)

## [v0.8.7](https://github.com/input-output-hk/jormungandr/tree/v0.8.7) (2020-01-23)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.6...v0.8.7)

**Implemented enhancements:**

- Added p2p stats to node stat output [\#1611](https://github.com/input-output-hk/jormungandr/pull/1611)
- cleanup the sync Mutex from the PeerMap [\#1594](https://github.com/input-output-hk/jormungandr/pull/1594)
- keep more room for nodes that are actually giving us blocks [\#1587](https://github.com/input-output-hk/jormungandr/pull/1587)
- Apply ref on each synchronized block during bootstrap [\#1582](https://github.com/input-output-hk/jormungandr/pull/1582)
- display the whole 'reason' of some errors with Debug formatting [\#1577](https://github.com/input-output-hk/jormungandr/pull/1577)
- Switch actix-web to rustls [\#1568](https://github.com/input-output-hk/jormungandr/pull/1568)

**Fixed bugs:**

- Rest - `/api/v0/shutdown` - doesn't shut the node down, just the rest service [\#1563](https://github.com/input-output-hk/jormungandr/issues/1563)
- use path instead of open option for in-memory sqlite [\#1614](https://github.com/input-output-hk/jormungandr/pull/1614)
- Fix node not shutting down when inner process finishes [\#1605](https://github.com/input-output-hk/jormungandr/pull/1605)

**Closed issues:**

- Configuration issue with Windows version of Jormungandr poolsecret1.yaml [\#1618](https://github.com/input-output-hk/jormungandr/issues/1618)
- It still gets stuck [\#1615](https://github.com/input-output-hk/jormungandr/issues/1615)
- Add Peer Stats to Node Stats Output [\#1610](https://github.com/input-output-hk/jormungandr/issues/1610)
- Misleading documentation for max_connections_threshold default value [\#1602](https://github.com/input-output-hk/jormungandr/issues/1602)
- Jormungandr Install issues on NixOS [\#1600](https://github.com/input-output-hk/jormungandr/issues/1600)
- Can a setting be added to throttle blocks uploaded? [\#1595](https://github.com/input-output-hk/jormungandr/issues/1595)
- remove the sync Mutex to an async Mutex in the PeerMap collection [\#1591](https://github.com/input-output-hk/jormungandr/issues/1591)
- Ticker not appear [\#1590](https://github.com/input-output-hk/jormungandr/issues/1590)
- data consistent between node and chain [\#1571](https://github.com/input-output-hk/jormungandr/issues/1571)
- Consider Rebranding "jormungandr" to "cardano-node-rust" [\#1562](https://github.com/input-output-hk/jormungandr/issues/1562)
- make public address mandatory for stake pool operators [\#1537](https://github.com/input-output-hk/jormungandr/issues/1537)
- MultiAddress invalid public_address [\#1519](https://github.com/input-output-hk/jormungandr/issues/1519)
- Lowlevel network improvements [\#1489](https://github.com/input-output-hk/jormungandr/issues/1489)
- Network Thread Panic at Shutdown [\#1466](https://github.com/input-output-hk/jormungandr/issues/1466)

**Merged pull requests:**

- Async-friendly lock on P2P topology [\#1623](https://github.com/input-output-hk/jormungandr/pull/1623)
- \[Tests\] compilation fix for perf tests [\#1619](https://github.com/input-output-hk/jormungandr/pull/1619)
- \[Tests\] test_jormungandr_passive_node_starts_successfull fix [\#1616](https://github.com/input-output-hk/jormungandr/pull/1616)
- Upgrade Actix-web to 2.0 [\#1613](https://github.com/input-output-hk/jormungandr/pull/1613)
- Update actix-web to 1.0 [\#1606](https://github.com/input-output-hk/jormungandr/pull/1606)
- \[Tests\] Private network tests - logging fix [\#1604](https://github.com/input-output-hk/jormungandr/pull/1604)
- Update network.md [\#1598](https://github.com/input-output-hk/jormungandr/pull/1598)
- \[Tests\] Remove genesis model duplication [\#1589](https://github.com/input-output-hk/jormungandr/pull/1589)
- \[Tests\] Remove linear fees duplication [\#1588](https://github.com/input-output-hk/jormungandr/pull/1588)
- \[Tests\]\[Perf tests\] updated tests parameters [\#1584](https://github.com/input-output-hk/jormungandr/pull/1584)
- \[Tests\] Removed duplicated Funds struct from jormungandr-integration-tests [\#1583](https://github.com/input-output-hk/jormungandr/pull/1583)
- \[Tests\] Refresh grpc mock tests [\#1578](https://github.com/input-output-hk/jormungandr/pull/1578)
- use a global runtime instead of a local one and new future [\#1572](https://github.com/input-output-hk/jormungandr/pull/1572)
- \[Tests\] extended bootstrap timeout for testnet integration test [\#1567](https://github.com/input-output-hk/jormungandr/pull/1567)
- Added common vscode and idea workspace folders to gitignore [\#1566](https://github.com/input-output-hk/jormungandr/pull/1566)

## [v0.8.6](https://github.com/input-output-hk/jormungandr/tree/v0.8.6) (2020-01-15)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.5...v0.8.6)

**Implemented enhancements:**

- Get rid of CandidateForest [\#1569](https://github.com/input-output-hk/jormungandr/pull/1569)
- Update chain-deps for rand and network API changes [\#1507](https://github.com/input-output-hk/jormungandr/pull/1507)
- Implement a TCP server connection manager that rejects incoming connections when full [\#1497](https://github.com/input-output-hk/jormungandr/pull/1497)

**Fixed bugs:**

- android compilation fails with rlimit [\#1553](https://github.com/input-output-hk/jormungandr/issues/1553)
- SqliteFailure: "no such table: Blocks" [\#1485](https://github.com/input-output-hk/jormungandr/issues/1485)
- Tests are sporadically failing on Circle CI [\#1463](https://github.com/input-output-hk/jormungandr/issues/1463)
- 0.8.5-alpha1 panics observed [\#1422](https://github.com/input-output-hk/jormungandr/issues/1422)
- no data at API endpoint /api/v0/stake_pool [\#1421](https://github.com/input-output-hk/jormungandr/issues/1421)
- Error in documentation of public_id [\#1420](https://github.com/input-output-hk/jormungandr/issues/1420)
- disable diagnostic on Android [\#1557](https://github.com/input-output-hk/jormungandr/pull/1557)
- replace task key with subtask for topology policy [\#1533](https://github.com/input-output-hk/jormungandr/pull/1533)
- enable shared cache for in-memory databases [\#1508](https://github.com/input-output-hk/jormungandr/pull/1508)
- enforce the correct order of writes to storage [\#1441](https://github.com/input-output-hk/jormungandr/pull/1441)

**Closed issues:**

- thread 'tokio-runtime-worker-1' panicked at 'called `Option::unwrap\(\)` [\#1564](https://github.com/input-output-hk/jormungandr/issues/1564)
- Upgrade to Async/Await: use tokio compat Runtime to allow new async/await use [\#1548](https://github.com/input-output-hk/jormungandr/issues/1548)
- Upgrade to Async/Await: use default runtime [\#1547](https://github.com/input-output-hk/jormungandr/issues/1547)
- CRIT task panicked [\#1542](https://github.com/input-output-hk/jormungandr/issues/1542)
- Cannot sign the block: This leader 1 is not in the enclave [\#1540](https://github.com/input-output-hk/jormungandr/issues/1540)
- Error processing incoming header stream -Mac OS Mojave [\#1535](https://github.com/input-output-hk/jormungandr/issues/1535)
- "Block theft" seems to be happening [\#1532](https://github.com/input-output-hk/jormungandr/issues/1532)
- Jormungandr - logs - poldercast / policy log task / sub_task [\#1499](https://github.com/input-output-hk/jormungandr/issues/1499)
- restart of SSH terminal causes node to stop syncing [\#1480](https://github.com/input-output-hk/jormungandr/issues/1480)
- Blockchain is not moving up - jormungandr v0.8.5 [\#1479](https://github.com/input-output-hk/jormungandr/issues/1479)
- Run jormungandr with version testnet Byron [\#1477](https://github.com/input-output-hk/jormungandr/issues/1477)
- Ghost block [\#1464](https://github.com/input-output-hk/jormungandr/issues/1464)
- CRIT Task panicked, task: leadership [\#1451](https://github.com/input-output-hk/jormungandr/issues/1451)
- v0.8.5 labeled as a Pre-release [\#1450](https://github.com/input-output-hk/jormungandr/issues/1450)
- stats: Server Error: 500 Internal Server Error [\#1449](https://github.com/input-output-hk/jormungandr/issues/1449)
- Leader log should have a flag if block was invalidated [\#1446](https://github.com/input-output-hk/jormungandr/issues/1446)
- Dec 25 00:51:54.086 WARN blockchain is not moving up, the last block was 4963 seconds ago, task: stuck_notifier [\#1443](https://github.com/input-output-hk/jormungandr/issues/1443)
- The Node is Not in Sync v0.8.5 [\#1440](https://github.com/input-output-hk/jormungandr/issues/1440)
- Node crash [\#1434](https://github.com/input-output-hk/jormungandr/issues/1434)
- itn_rewards_v1 - jcli generates a binary genesis which hash is different from the official one [\#1430](https://github.com/input-output-hk/jormungandr/issues/1430)
- Block produced but is not visible in Cardano Explorer [\#1427](https://github.com/input-output-hk/jormungandr/issues/1427)
- the node is not synced v0.8.3 [\#1410](https://github.com/input-output-hk/jormungandr/issues/1410)
- CRIT Task panicked, task: block [\#1408](https://github.com/input-output-hk/jormungandr/issues/1408)
- ERRO cannot compute the time passed [\#1404](https://github.com/input-output-hk/jormungandr/issues/1404)
- Wrong parent block - j. 0.8.3 [\#1403](https://github.com/input-output-hk/jormungandr/issues/1403)
- The lastBlockTime increases but lastBlockHeight freezes [\#1402](https://github.com/input-output-hk/jormungandr/issues/1402)
- Error: deadline has elapsed [\#1392](https://github.com/input-output-hk/jormungandr/issues/1392)

**Merged pull requests:**

- Revert "use a global runtime instead of a local one and new future" [\#1570](https://github.com/input-output-hk/jormungandr/pull/1570)
- remove unused shell.nix file [\#1559](https://github.com/input-output-hk/jormungandr/pull/1559)
- \[Tests\] Disruption private network test [\#1558](https://github.com/input-output-hk/jormungandr/pull/1558)
- use a global runtime instead of a local one and new future [\#1556](https://github.com/input-output-hk/jormungandr/pull/1556)
- Dockerfile uses VER as an ARG but not VERSION [\#1555](https://github.com/input-output-hk/jormungandr/pull/1555)
- miss to cp a shell script [\#1554](https://github.com/input-output-hk/jormungandr/pull/1554)
- integrate new SQLiteBlockStore constructors [\#1550](https://github.com/input-output-hk/jormungandr/pull/1550)
- use `with\_executor` as per breaking changes from chain-libs/network-grpc [\#1549](https://github.com/input-output-hk/jormungandr/pull/1549)
- use one Runtime for all the services. [\#1546](https://github.com/input-output-hk/jormungandr/pull/1546)
- Remove some dead code [\#1545](https://github.com/input-output-hk/jormungandr/pull/1545)
- Make server connection failures non-fatal [\#1531](https://github.com/input-output-hk/jormungandr/pull/1531)
- rearrange sections, simplify text [\#1528](https://github.com/input-output-hk/jormungandr/pull/1528)
- \[Tests\] Jts soak test [\#1525](https://github.com/input-output-hk/jormungandr/pull/1525)
- Thread pool for server tasks [\#1523](https://github.com/input-output-hk/jormungandr/pull/1523)
- add RELEASE file [\#1520](https://github.com/input-output-hk/jormungandr/pull/1520)
- \[Tests\] Reward test fix [\#1516](https://github.com/input-output-hk/jormungandr/pull/1516)
- \[Test\] Implement long soak test \(Selfnode\) [\#1515](https://github.com/input-output-hk/jormungandr/pull/1515)
- Docs: configuration/network - gossip_interval - fix default value [\#1506](https://github.com/input-output-hk/jormungandr/pull/1506)
- \[Tests\] NodeStats struct update [\#1487](https://github.com/input-output-hk/jormungandr/pull/1487)
- Command for generating the public id [\#1486](https://github.com/input-output-hk/jormungandr/pull/1486)
- Clean up docs for network public_id [\#1484](https://github.com/input-output-hk/jormungandr/pull/1484)
- Fix stake pool OpenAPI lacking pool_id parameter [\#1483](https://github.com/input-output-hk/jormungandr/pull/1483)
- \[Test\] Testnet: added "Port already in use" new error code [\#1482](https://github.com/input-output-hk/jormungandr/pull/1482)
- \[Tests\] build fix for explorer pr [\#1481](https://github.com/input-output-hk/jormungandr/pull/1481)
- \[Test\] Collect reward test fix [\#1478](https://github.com/input-output-hk/jormungandr/pull/1478)
- \[Tests\] Explorer test [\#1414](https://github.com/input-output-hk/jormungandr/pull/1414)

## [v0.8.5](https://github.com/input-output-hk/jormungandr/tree/v0.8.5) (2019-12-23)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.4...v0.8.5)

**Implemented enhancements:**

- leverage SQLite multi-threading power by removing the bottleneck lock [\#1412](https://github.com/input-output-hk/jormungandr/pull/1412)

**Fixed bugs:**

- Fix block time issue in the REST API and documentation [\#1426](https://github.com/input-output-hk/jormungandr/pull/1426)
- Handle concurrency in CandidateForest::apply_block [\#1425](https://github.com/input-output-hk/jormungandr/pull/1425)
- Thread 'blockX' panicked at missed chain pull root candidate [\#1388](https://github.com/input-output-hk/jormungandr/issues/1388)
- blockchain: More robust CandidateForest [\#1405](https://github.com/input-output-hk/jormungandr/pull/1405)

**Closed issues:**

- CRIT Service has terminated with an error [\#1418](https://github.com/input-output-hk/jormungandr/issues/1418)

**Merged pull requests:**

- \[Tests\] Stability fix - Change reward test parameters [\#1393](https://github.com/input-output-hk/jormungandr/pull/1393)

## [v0.8.5-alpha3](https://github.com/input-output-hk/jormungandr/tree/v0.8.5-alpha3) (2019-12-22)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.5-alpha2...v0.8.5-alpha3)

**Fixed bugs:**

- Fix block time issue in the REST API and documentation [\#1426](https://github.com/input-output-hk/jormungandr/pull/1426)

## [v0.8.5-alpha2](https://github.com/input-output-hk/jormungandr/tree/v0.8.5-alpha2) (2019-12-21)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.5-alpha1...v0.8.5-alpha2)

**Fixed bugs:**

- Handle concurrency in CandidateForest::apply_block [\#1425](https://github.com/input-output-hk/jormungandr/pull/1425)

**Closed issues:**

- CRIT Service has terminated with an error [\#1418](https://github.com/input-output-hk/jormungandr/issues/1418)

## [v0.8.5-alpha1](https://github.com/input-output-hk/jormungandr/tree/v0.8.5-alpha1) (2019-12-20)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.4...v0.8.5-alpha1)

**Implemented enhancements:**

- leverage SQLite multi-threading power by removing the bottleneck lock [\#1412](https://github.com/input-output-hk/jormungandr/pull/1412)

**Fixed bugs:**

- Thread 'blockX' panicked at missed chain pull root candidate [\#1388](https://github.com/input-output-hk/jormungandr/issues/1388)
- blockchain: More robust CandidateForest [\#1405](https://github.com/input-output-hk/jormungandr/pull/1405)

**Merged pull requests:**

- \[Tests\] Stability fix - Change reward test parameters [\#1393](https://github.com/input-output-hk/jormungandr/pull/1393)

## [v0.8.4](https://github.com/input-output-hk/jormungandr/tree/v0.8.4) (2019-12-19)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.3...v0.8.4)

**Fixed bugs:**

- Panic: transaction not found for utxo input [\#1382](https://github.com/input-output-hk/jormungandr/issues/1382)
- Panic on 'invalid key' [\#1381](https://github.com/input-output-hk/jormungandr/issues/1381)
- Question: initial bootstrap failed, error: BlockMissingParent... referring to block0, but wrong parent. [\#1326](https://github.com/input-output-hk/jormungandr/issues/1326)
- Eliminate duplicates in transactions by address [\#1397](https://github.com/input-output-hk/jormungandr/pull/1397)
- handle utxo pointers in the same block [\#1394](https://github.com/input-output-hk/jormungandr/pull/1394)
- Fix a race condition of header pull with block pull [\#1389](https://github.com/input-output-hk/jormungandr/pull/1389)
- Exclude block 0 from sent branches [\#1385](https://github.com/input-output-hk/jormungandr/pull/1385)

**Closed issues:**

- peer node ID differs from the expected X hash, node id: Y hash [\#1390](https://github.com/input-output-hk/jormungandr/issues/1390)
- Compilation error building latest master on MacOS [\#1376](https://github.com/input-output-hk/jormungandr/issues/1376)
- Limit concurrent pull requests [\#1349](https://github.com/input-output-hk/jormungandr/issues/1349)

**Merged pull requests:**

- Limit concurrent pull requests [\#1365](https://github.com/input-output-hk/jormungandr/pull/1365)
- remove custom_error [\#1345](https://github.com/input-output-hk/jormungandr/pull/1345)

## [v0.8.3](https://github.com/input-output-hk/jormungandr/tree/v0.8.3) (2019-12-17)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.2...v0.8.3)

**Fixed bugs:**

- Node stuck - block is already cached as a candidate - panic 'assertion failed: \_old.is_none\(\)' - immediate memory increase [\#1327](https://github.com/input-output-hk/jormungandr/issues/1327)
- jcli transaction make-witness fails with error [\#1323](https://github.com/input-output-hk/jormungandr/issues/1323)

**Closed issues:**

- jcli rest v0 stake get no longer lists all pools with delegated stake in 0.8.2 [\#1371](https://github.com/input-output-hk/jormungandr/issues/1371)
- where is jcli? [\#1369](https://github.com/input-output-hk/jormungandr/issues/1369)
- Error in the overall configuration of the node/ bootstrap file. v0.8.2 [\#1363](https://github.com/input-output-hk/jormungandr/issues/1363)
- 404 Not Found status code is too generic for unused addresses which confuses jcli users [\#1361](https://github.com/input-output-hk/jormungandr/issues/1361)

**Merged pull requests:**

- \[Tests\] Another Fix for non functional tests [\#1378](https://github.com/input-output-hk/jormungandr/pull/1378)
- \[Tests\] fixed non_functional tests compilation issues [\#1375](https://github.com/input-output-hk/jormungandr/pull/1375)
- Reverts change in 7e79334da1a46c484e3d6ffe0c52e2518c3c4c44 which remo… [\#1362](https://github.com/input-output-hk/jormungandr/pull/1362)
- Docs: jcli/transaction + stake_pool/delegating_stake - fix [\#1347](https://github.com/input-output-hk/jormungandr/pull/1347)
- Rework chain pull to be concurrency friendly [\#1346](https://github.com/input-output-hk/jormungandr/pull/1346)

## [v0.8.2](https://github.com/input-output-hk/jormungandr/tree/v0.8.2) (2019-12-13)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.1...v0.8.2)

**Merged pull requests:**

- chain-deps update [\#1359](https://github.com/input-output-hk/jormungandr/pull/1359)

## [v0.8.1](https://github.com/input-output-hk/jormungandr/tree/v0.8.1) (2019-12-13)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0...v0.8.1)

**Implemented enhancements:**

- Minor enhancements to network block processing [\#1353](https://github.com/input-output-hk/jormungandr/pull/1353)

**Closed issues:**

- make-witness account transaction error trying to register pool [\#1344](https://github.com/input-output-hk/jormungandr/issues/1344)
- The transaction fees should not be included into the rewards for the Incentivized testnet [\#1340](https://github.com/input-output-hk/jormungandr/issues/1340)
- Rewards are not evenly distributed per stake pool based on the number of blocks created in an epoch - 0.8.0-RC10 [\#1325](https://github.com/input-output-hk/jormungandr/issues/1325)

**Merged pull requests:**

- fix fees application for owner stake delegation [\#1357](https://github.com/input-output-hk/jormungandr/pull/1357)

## [v0.8.0](https://github.com/input-output-hk/jormungandr/tree/v0.8.0) (2019-12-11)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc11...v0.8.0)

**Implemented enhancements:**

- GraphQL: expose treasury balance [\#1247](https://github.com/input-output-hk/jormungandr/issues/1247)
- Graphql treasury balance and settings [\#1342](https://github.com/input-output-hk/jormungandr/pull/1342)
- add reward constraints parameter [\#1338](https://github.com/input-output-hk/jormungandr/pull/1338)
- Debug block operation in jcli [\#1337](https://github.com/input-output-hk/jormungandr/pull/1337)

**Fixed bugs:**

- rest node stats does not count/consider all kinds of tx/fragments [\#1301](https://github.com/input-output-hk/jormungandr/issues/1301)
- Get REST node stats from all framents containing TX [\#1343](https://github.com/input-output-hk/jormungandr/pull/1343)

**Closed issues:**

- Add jcli command / rest endpoint for decoding a block [\#1336](https://github.com/input-output-hk/jormungandr/issues/1336)

**Merged pull requests:**

- \[Tests\] logging enhancements for Private network tests [\#1339](https://github.com/input-output-hk/jormungandr/pull/1339)

## [v0.8.0-rc11](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc11) (2019-12-10)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc10...v0.8.0-rc11)

**Implemented enhancements:**

- jcli help message to improve and error message to be more precise [\#1310](https://github.com/input-output-hk/jormungandr/issues/1310)
- Add node IPs in REST network stats [\#1261](https://github.com/input-output-hk/jormungandr/issues/1261)
- Expose Poldercast and Node Quarantine status [\#1332](https://github.com/input-output-hk/jormungandr/pull/1332)
- Add node IPs in REST network stats [\#1331](https://github.com/input-output-hk/jormungandr/pull/1331)
- blockchain: Purge candidate forest from unresolved branches [\#1329](https://github.com/input-output-hk/jormungandr/pull/1329)
- create directory if it does not exist [\#1328](https://github.com/input-output-hk/jormungandr/pull/1328)
- jcli key derive - update help messages related to bip32 keys [\#1315](https://github.com/input-output-hk/jormungandr/pull/1315)

**Fixed bugs:**

- 0.8.0-RC9+1 - value_taxed \> TAX_LIMIT for stake pool [\#1304](https://github.com/input-output-hk/jormungandr/issues/1304)
- fix open-api document [\#1322](https://github.com/input-output-hk/jormungandr/pull/1322)
- remove extra line in csv dump [\#1321](https://github.com/input-output-hk/jormungandr/pull/1321)
- remove trailing space to be compatible RFC4180 [\#1318](https://github.com/input-output-hk/jormungandr/pull/1318)
- remove duplicated line [\#1316](https://github.com/input-output-hk/jormungandr/pull/1316)
- Fix jcli doc on tax [\#1313](https://github.com/input-output-hk/jormungandr/pull/1313)

**Merged pull requests:**

- \[Tests\] Rewards integration tests [\#1330](https://github.com/input-output-hk/jormungandr/pull/1330)
- more fixes in the openapi doc [\#1324](https://github.com/input-output-hk/jormungandr/pull/1324)
- Update registering_stake_pool.md [\#1317](https://github.com/input-output-hk/jormungandr/pull/1317)

## [v0.8.0-rc10](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc10) (2019-12-09)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc9...v0.8.0-rc10)

**Implemented enhancements:**

- Update `get settings` output \(maxTxsPerBlock, block_content_max_size\) - 0..8.0-RC9 [\#1298](https://github.com/input-output-hk/jormungandr/issues/1298)
- REST: update /api/v0/settings [\#1248](https://github.com/input-output-hk/jormungandr/issues/1248)
- dump the data in csv so easier to process on the long run [\#1311](https://github.com/input-output-hk/jormungandr/pull/1311)
- write reward_info to a file at rewards creation AND fees_go_to settings in the genesis yaml file [\#1307](https://github.com/input-output-hk/jormungandr/pull/1307)
- Rest: settings/stats - update and cleanup [\#1299](https://github.com/input-output-hk/jormungandr/pull/1299)
- change to old sqlite connection impl [\#1294](https://github.com/input-output-hk/jormungandr/pull/1294)
- Add reward and treasury settings to settings REST [\#1291](https://github.com/input-output-hk/jormungandr/pull/1291)

**Fixed bugs:**

- Fix build on FreeBSD [\#1302](https://github.com/input-output-hk/jormungandr/pull/1302)

**Merged pull requests:**

- prevent invalid addr to be set in the poldercast entry [\#1309](https://github.com/input-output-hk/jormungandr/pull/1309)
- \[Tests\] sync spending counter with blockchain [\#1306](https://github.com/input-output-hk/jormungandr/pull/1306)
- \[Tests\] Update node stats dao in jormungandr_lib [\#1300](https://github.com/input-output-hk/jormungandr/pull/1300)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc8...v0.8.0-rc9)

**Breaking changes:**

- remove useless block0 parameter: BFT Slots Ratio [\#1293](https://github.com/input-output-hk/jormungandr/pull/1293)
- BlockContent Size: finally set the right value for number of fragments [\#1288](https://github.com/input-output-hk/jormungandr/pull/1288)

**Implemented enhancements:**

- Add proper depth [\#1295](https://github.com/input-output-hk/jormungandr/pull/1295)
- spawn blockchain process to allow for more concurrent action to happen [\#1290](https://github.com/input-output-hk/jormungandr/pull/1290)

**Fixed bugs:**

- jormungandr v0.8.0-rc7 got stuck on synchronisation [\#1284](https://github.com/input-output-hk/jormungandr/issues/1284)
- jormungandr 0.8.0-rc5-cecea4d got stuck on synchronisation [\#1273](https://github.com/input-output-hk/jormungandr/issues/1273)
- Spamming the test net appears to break it or at least cause multiple node stalls long after the spamming stops [\#1235](https://github.com/input-output-hk/jormungandr/issues/1235)
- 0.7.5 \(or nightly testnet\) - generated blocks are not added to the blockchain [\#1221](https://github.com/input-output-hk/jormungandr/issues/1221)
- 0.7.4 - Blocks not getting added to chain [\#1220](https://github.com/input-output-hk/jormungandr/issues/1220)
- 0.7.1 Frequent Warning - WARN blockchain is not moving up.... [\#1183](https://github.com/input-output-hk/jormungandr/issues/1183)
- prevent panic if the given chain advance is removed from concurrent processing [\#1296](https://github.com/input-output-hk/jormungandr/pull/1296)

**Closed issues:**

- 0.8.0-rc2 - Node still shutting down on beta testnet [\#1234](https://github.com/input-output-hk/jormungandr/issues/1234)
- 0.7.1 - Error processing ChainHeader handling [\#1179](https://github.com/input-output-hk/jormungandr/issues/1179)

**Merged pull requests:**

- Spawn client processing in tasks [\#1292](https://github.com/input-output-hk/jormungandr/pull/1292)
- Fix mismatch step number [\#1289](https://github.com/input-output-hk/jormungandr/pull/1289)

## [v0.8.0-rc8](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc8) (2019-12-05)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc7...v0.8.0-rc8)

**Implemented enhancements:**

- add in more logs in the block event handling [\#1287](https://github.com/input-output-hk/jormungandr/pull/1287)
- Fragment pool boundaries [\#1285](https://github.com/input-output-hk/jormungandr/pull/1285)
- Remove unimplemented! in protocol request handlers. [\#1280](https://github.com/input-output-hk/jormungandr/pull/1280)
- blockchain: Rework task state [\#1279](https://github.com/input-output-hk/jormungandr/pull/1279)

**Fixed bugs:**

- Downloading data on incentivized node is very slowly [\#1262](https://github.com/input-output-hk/jormungandr/issues/1262)
- little fix in the reward calculation [\#1283](https://github.com/input-output-hk/jormungandr/pull/1283)

**Closed issues:**

- Rewards for the current epoch are allocated at the beginning of the epoch - 0.8.0-RC7 [\#1282](https://github.com/input-output-hk/jormungandr/issues/1282)

## [v0.8.0-rc7](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc7) (2019-12-04)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc6...v0.8.0-rc7)

**Implemented enhancements:**

- Don't thrash slow connections in propagation [\#1277](https://github.com/input-output-hk/jormungandr/pull/1277)

**Fixed bugs:**

- No rewards received - 0.8.0-RC6 - local cluster 2 nodes [\#1275](https://github.com/input-output-hk/jormungandr/issues/1275)
- 0.8 rc6 - delegator accounts \(standalone\) still not getting paid rewards [\#1274](https://github.com/input-output-hk/jormungandr/issues/1274)
- update chain-deps and include a fix in the delegators reward distribution [\#1276](https://github.com/input-output-hk/jormungandr/pull/1276)

## [v0.8.0-rc6](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc6) (2019-12-03)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc5...v0.8.0-rc6)

**Fixed bugs:**

- Neither delegatee nor delegator are getting rewards - 0.8.0-rc5 [\#1271](https://github.com/input-output-hk/jormungandr/issues/1271)
- update chain deps to include fixes on the delegation [\#1272](https://github.com/input-output-hk/jormungandr/pull/1272)

## [v0.8.0-rc5](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc5) (2019-12-03)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc4...v0.8.0-rc5)

**Breaking changes:**

- change how to set the reward account in the jcli command line parameter [\#1259](https://github.com/input-output-hk/jormungandr/pull/1259)

**Implemented enhancements:**

- detect node's environment/system settings at boot or on demand [\#1215](https://github.com/input-output-hk/jormungandr/issues/1215)
- return stake-pools public VRF key [\#1163](https://github.com/input-output-hk/jormungandr/issues/1163)
- fix reward distribution and expose the rewards in the REST API [\#1269](https://github.com/input-output-hk/jormungandr/pull/1269)
- Add version to REST node stats [\#1265](https://github.com/input-output-hk/jormungandr/pull/1265)
- Resolve the ancestor once for chain streaming [\#1258](https://github.com/input-output-hk/jormungandr/pull/1258)
- Don't panic on network task error [\#1255](https://github.com/input-output-hk/jormungandr/pull/1255)
- Expose resource usage limits on UNIX systems [\#1222](https://github.com/input-output-hk/jormungandr/pull/1222)

**Fixed bugs:**

- 0.8 rc4 - standalone delegators not getting paid rewards after pool owners/operators got their tax cut [\#1250](https://github.com/input-output-hk/jormungandr/issues/1250)
- fix rlimit builds on different libc impls [\#1267](https://github.com/input-output-hk/jormungandr/pull/1267)
- Yield the task after retrieving each block to send [\#1264](https://github.com/input-output-hk/jormungandr/pull/1264)
- don't fail on error in the client task [\#1249](https://github.com/input-output-hk/jormungandr/pull/1249)

**Closed issues:**

- logging improvement epoch.block time [\#1251](https://github.com/input-output-hk/jormungandr/issues/1251)

**Merged pull requests:**

- Rest node stats REST cert fees [\#1266](https://github.com/input-output-hk/jormungandr/pull/1266)
- add documentation regarding the stake pool Tax [\#1257](https://github.com/input-output-hk/jormungandr/pull/1257)

## [v0.8.0-rc4](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc4) (2019-12-02)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc3...v0.8.0-rc4)

**Fixed bugs:**

- 0.8 rc3 - rewards still not working due to leader_logs.total or subsequent panic [\#1242](https://github.com/input-output-hk/jormungandr/issues/1242)
- update chain-deps and fix reward distribution panic [\#1246](https://github.com/input-output-hk/jormungandr/pull/1246)
- properly report error and failures of the terminating service [\#1243](https://github.com/input-output-hk/jormungandr/pull/1243)

**Closed issues:**

- bootstrap.sh - error: The following required arguments were not provided: --serial \<SERIAL\> [\#1244](https://github.com/input-output-hk/jormungandr/issues/1244)

**Merged pull requests:**

- don't run test on appveyor PRs [\#1245](https://github.com/input-output-hk/jormungandr/pull/1245)

**Breaking changes:**

- update chain-deps and changed the UTxO signature [\#1246](https://github.com/input-output-hk/jormungandr/pull/1246)

## [v0.8.0-rc3](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc3) (2019-12-01)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc2...v0.8.0-rc3)

**Implemented enhancements:**

- don't add the block0 per-certificate fee if they are all not set (0) [\#1239](https://github.com/input-output-hk/jormungandr/pull/1239)

**Fixed bugs:**

- 0.8 rc2 - rewards not being paid out as expected \(private testnet\) [\#1237](https://github.com/input-output-hk/jormungandr/issues/1237)
- Node shutdowns are classified at wrong priority and exit successful [\#1236](https://github.com/input-output-hk/jormungandr/issues/1236)
- make the node actually return an error if a service was stopped because of error [\#1240](https://github.com/input-output-hk/jormungandr/pull/1240)
- don't fail the stuck notifier task if time is set in the future [\#1241](https://github.com/input-output-hk/jormungandr/pull/1241)

**Breaking changes:**

- apply the reward to a transition_state and keep it for the safe keeping [\#1238](https://github.com/input-output-hk/jormungandr/pull/1238)

## [v0.8.0-rc2](https://github.com/input-output-hk/jormungandr/tree/v0.8.0-rc2) (2019-11-30)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.8.0-rc1...v0.8.0-rc2)

**Implemented enhancements:**

- Log which service has finished and how [\#1230](https://github.com/input-output-hk/jormungandr/pull/1230)
- Implement all stake pools graphql query [\#1223](https://github.com/input-output-hk/jormungandr/pull/1223)

**Fixed bugs:**

- intercom: Make reply not fatal if receiver goes away [\#1233](https://github.com/input-output-hk/jormungandr/pull/1233)

**Closed issues:**

- 0.8.0-rc1: shutdowns, stability issues [\#1232](https://github.com/input-output-hk/jormungandr/issues/1232)
- From BIP39 private key to Ed25519 [\#1211](https://github.com/input-output-hk/jormungandr/issues/1211)

**Merged pull requests:**

- Task unwind safety [\#1231](https://github.com/input-output-hk/jormungandr/pull/1231)
- Switch Circle-CI from Rust nightly to beta [\#1229](https://github.com/input-output-hk/jormungandr/pull/1229)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.5...v0.8.0-rc1)

**Breaking changes:**

- update to latest chain-deps: add incentive [\#1193](https://github.com/input-output-hk/jormungandr/pull/1193)
- Certificate fees [\#1191](https://github.com/input-output-hk/jormungandr/pull/1191)

**Implemented enhancements:**

- Add owner stake deletagion cert creation tool to JCLI [\#1202](https://github.com/input-output-hk/jormungandr/issues/1202)
- expose the reward parameters [\#1227](https://github.com/input-output-hk/jormungandr/pull/1227)
- Pull from block 0 if no checkpoints intersect [\#1225](https://github.com/input-output-hk/jormungandr/pull/1225)
- Add owner stake deletagion cert creation tool to JCLI [\#1224](https://github.com/input-output-hk/jormungandr/pull/1224)
- Allow setting treasury in the genesis file [\#1213](https://github.com/input-output-hk/jormungandr/pull/1213)
- Add input output to block and initial fees [\#1198](https://github.com/input-output-hk/jormungandr/pull/1198)
- Add multisignature to address in explorer [\#1197](https://github.com/input-output-hk/jormungandr/pull/1197)
- remote syslog via UDP [\#1196](https://github.com/input-output-hk/jormungandr/pull/1196)
- Add stake pool details getter to REST [\#1195](https://github.com/input-output-hk/jormungandr/pull/1195)
- Small network fixes, improve logging [\#1194](https://github.com/input-output-hk/jormungandr/pull/1194)

**Fixed bugs:**

- Fix nightly for introduction of built-in never type [\#1228](https://github.com/input-output-hk/jormungandr/pull/1228)
- update chain-libs, include fix for osx mbi1 [\#1226](https://github.com/input-output-hk/jormungandr/pull/1226)
- Header chain validation errors are not fatal [\#1218](https://github.com/input-output-hk/jormungandr/pull/1218)
- mitigate issue with loading existing state from storage [\#1214](https://github.com/input-output-hk/jormungandr/pull/1214)
- Fix per certificate fees and APIs [\#1212](https://github.com/input-output-hk/jormungandr/pull/1212)
- fix stake pool blocks query off by one [\#1205](https://github.com/input-output-hk/jormungandr/pull/1205)

**Closed issues:**

- 0.7.3-0.7.4, error compiling jormungandr-lib v0.7.3-0.7.4 [\#1217](https://github.com/input-output-hk/jormungandr/issues/1217)

**Merged pull requests:**

- Add `git submodule update` to "How to install from sources" [\#1219](https://github.com/input-output-hk/jormungandr/pull/1219)
- Update doc [\#1210](https://github.com/input-output-hk/jormungandr/pull/1210)
- Optimize locking with Storage::send_from_to [\#1209](https://github.com/input-output-hk/jormungandr/pull/1209)
- rename blockchain_stuck_notifier [\#1208](https://github.com/input-output-hk/jormungandr/pull/1208)
- \[Tests\] Jts timeout fix [\#1207](https://github.com/input-output-hk/jormungandr/pull/1207)
- Disgraceful REST shutdown [\#1203](https://github.com/input-output-hk/jormungandr/pull/1203)
- \[Tests\] another attempt to stabilize tests [\#1199](https://github.com/input-output-hk/jormungandr/pull/1199)
- README.md: typo [\#1181](https://github.com/input-output-hk/jormungandr/pull/1181)
- \[Tests\]\[Testnet\] more logging for testnet test [\#1177](https://github.com/input-output-hk/jormungandr/pull/1177)
- Transform REST server into Tokio service [\#1173](https://github.com/input-output-hk/jormungandr/pull/1173)

## [v0.7.2](https://github.com/input-output-hk/jormungandr/tree/v0.7.2) (2019-11-25)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.1...v0.7.2)

**Implemented enhancements:**

- Expose the parameters of the stake pool in the graphQL data [\#1158](https://github.com/input-output-hk/jormungandr/issues/1158)
- Convert the client task to async, use bounded channels in intercom [\#1178](https://github.com/input-output-hk/jormungandr/pull/1178)

**Closed issues:**

- 0.7.1 Node startup fails with logging options in config [\#1184](https://github.com/input-output-hk/jormungandr/issues/1184)
- bootstrap script to create accounts with error [\#1182](https://github.com/input-output-hk/jormungandr/issues/1182)
- Discordant results between jcli and janalyze about leadership stats [\#1176](https://github.com/input-output-hk/jormungandr/issues/1176)
- Convert the client task to full async and remove the header pull limit [\#1160](https://github.com/input-output-hk/jormungandr/issues/1160)

**Merged pull requests:**

- Detect if `set -o pipefail` is available [\#1186](https://github.com/input-output-hk/jormungandr/pull/1186)
- \[Jormungandr-scenario-test\] fix failing tests from nighly run [\#1168](https://github.com/input-output-hk/jormungandr/pull/1168)
- regenerate grpc port after unsuccessful jormungandr bootstrap [\#1088](https://github.com/input-output-hk/jormungandr/pull/1088)

## [v0.7.1](https://github.com/input-output-hk/jormungandr/tree/v0.7.1) (2019-11-21)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.0...v0.7.1)

**Implemented enhancements:**

- add config for the number of allowed non reachable nodes at a time [\#1150](https://github.com/input-output-hk/jormungandr/issues/1150)
- Logging improvements for network subscriptions [\#1175](https://github.com/input-output-hk/jormungandr/pull/1175)
- poldercast crate update and automatic topology reset [\#1172](https://github.com/input-output-hk/jormungandr/pull/1172)
- Add pool retirement and pool update certificates graphql types [\#1169](https://github.com/input-output-hk/jormungandr/pull/1169)
- Expose stake pool parameters [\#1161](https://github.com/input-output-hk/jormungandr/pull/1161)
- Ground work for header chain validation [\#1159](https://github.com/input-output-hk/jormungandr/pull/1159)
- Add warning if blockchain is not moving up [\#1157](https://github.com/input-output-hk/jormungandr/pull/1157)
- allow setting the number of unreachable nodes to contact for propagation [\#1153](https://github.com/input-output-hk/jormungandr/pull/1153)
- Gracefully handle mutual connection flares [\#1139](https://github.com/input-output-hk/jormungandr/pull/1139)
- Update tip after network blocks [\#1138](https://github.com/input-output-hk/jormungandr/pull/1138)
- Implement BIP32 key derivation [\#1136](https://github.com/input-output-hk/jormungandr/pull/1136)
- Logging to multiple outputs [\#1134](https://github.com/input-output-hk/jormungandr/pull/1134)
- \(\#511\) logging to a file [\#1118](https://github.com/input-output-hk/jormungandr/pull/1118)

**Fixed bugs:**

- Timing issue in Jormungandr for slot leader signing blocks [\#1143](https://github.com/input-output-hk/jormungandr/issues/1143)
- Rise type_length_limit [\#1162](https://github.com/input-output-hk/jormungandr/pull/1162)
- catch the error the drains [\#1154](https://github.com/input-output-hk/jormungandr/pull/1154)
- Make sure if the node wake too early for the leader event to wait a bit [\#1151](https://github.com/input-output-hk/jormungandr/pull/1151)

**Closed issues:**

- Local 0.7.0 Jormungandr node not receiving blocks from Incentivized TestNet [\#1147](https://github.com/input-output-hk/jormungandr/issues/1147)
- bootstrap script fails [\#1142](https://github.com/input-output-hk/jormungandr/issues/1142)
- Which faucet to use for the latest release v0.7.0? [\#1135](https://github.com/input-output-hk/jormungandr/issues/1135)
- Error fetching the genesis block from the network [\#1132](https://github.com/input-output-hk/jormungandr/issues/1132)
- panic with error: Some\(NonMonotonicDate [\#1130](https://github.com/input-output-hk/jormungandr/issues/1130)
- Documentation to register a stake pool is not up to date [\#1110](https://github.com/input-output-hk/jormungandr/issues/1110)
- panicked at 'upper_bound should be \>= than lower_bound' [\#1093](https://github.com/input-output-hk/jormungandr/issues/1093)

**Merged pull requests:**

- added script for creating a new stakepool [\#1170](https://github.com/input-output-hk/jormungandr/pull/1170)
- clean imports in jormungandr-integration-tests [\#1167](https://github.com/input-output-hk/jormungandr/pull/1167)
- Shutdown node when any service terminates [\#1141](https://github.com/input-output-hk/jormungandr/pull/1141)
- Move REST to a service [\#1140](https://github.com/input-output-hk/jormungandr/pull/1140)
- Fetch block's body only once for transactions in block [\#1133](https://github.com/input-output-hk/jormungandr/pull/1133)
- Add safety checks to bootstrap script [\#1131](https://github.com/input-output-hk/jormungandr/pull/1131)
- \[Tests\]\[JST\] Improve Error Reporting [\#1129](https://github.com/input-output-hk/jormungandr/pull/1129)
- Docs: fix mdbook-linkcheck errors [\#1127](https://github.com/input-output-hk/jormungandr/pull/1127)
- make the blockchain::process fully async [\#1126](https://github.com/input-output-hk/jormungandr/pull/1126)
- Scripts: fix some issues related to certificates [\#1123](https://github.com/input-output-hk/jormungandr/pull/1123)
- Chain pull redux [\#1121](https://github.com/input-output-hk/jormungandr/pull/1121)
- logging settings: .async\(\) -\> .into_async\(\) [\#1119](https://github.com/input-output-hk/jormungandr/pull/1119)
- Clean up JCLI TX info command [\#1117](https://github.com/input-output-hk/jormungandr/pull/1117)
- Docs: update some certificate related commands [\#1113](https://github.com/input-output-hk/jormungandr/pull/1113)
- \[Tests\] Jormungandr test scenarios - additional test cases [\#1082](https://github.com/input-output-hk/jormungandr/pull/1082)

## [v0.7.0](https://github.com/input-output-hk/jormungandr/tree/v0.7.0) (2019-11-12)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.0-rc7...v0.7.0)

**Implemented enhancements:**

- Allow setting multiple stake pool for delegation [\#1089](https://github.com/input-output-hk/jormungandr/issues/1089)
- Leeway for schedule to happen [\#1114](https://github.com/input-output-hk/jormungandr/pull/1114)
- Allow setting multiple stake pool for delegation [\#1112](https://github.com/input-output-hk/jormungandr/pull/1112)
- leadership re-write [\#1106](https://github.com/input-output-hk/jormungandr/pull/1106)
- Properly filter checkpoints for given descendant [\#1100](https://github.com/input-output-hk/jormungandr/pull/1100)
- breaking change: add pool permission + operators [\#1097](https://github.com/input-output-hk/jormungandr/pull/1097)
- Start improving the readability of this document. [\#1083](https://github.com/input-output-hk/jormungandr/pull/1083)

**Fixed bugs:**

- explorer panic [\#1103](https://github.com/input-output-hk/jormungandr/issues/1103)
- node panicked - "cannot process leadership block" - cluster with 2 nodes on local pc; v0.7.0-rc4 [\#1065](https://github.com/input-output-hk/jormungandr/issues/1065)
- error while streaming response: Error { code: Internal, cause: CannotIterate }, sub_task: server, task: network [\#1056](https://github.com/input-output-hk/jormungandr/issues/1056)
- Excessive network/memory usage [\#1044](https://github.com/input-output-hk/jormungandr/issues/1044)
- Invalid block should not unwrap on the chain task and panic [\#1024](https://github.com/input-output-hk/jormungandr/issues/1024)
- Large amount of network traffic in short time frame. [\#1007](https://github.com/input-output-hk/jormungandr/issues/1007)
- Leeway for schedule to happen [\#1114](https://github.com/input-output-hk/jormungandr/pull/1114)
- leadership re-write [\#1106](https://github.com/input-output-hk/jormungandr/pull/1106)
- Fix blocks in epoch first cursor not being 0 [\#1096](https://github.com/input-output-hk/jormungandr/pull/1096)

**Closed issues:**

- Panic in jormungandr 0.7.0-rc7 [\#1105](https://github.com/input-output-hk/jormungandr/issues/1105)
- Re-open \#1094 [\#1104](https://github.com/input-output-hk/jormungandr/issues/1104)
- Port to Rust 2018 edition [\#1098](https://github.com/input-output-hk/jormungandr/issues/1098)
- Error in the overall configuration of the node |-\> Error while parsing the node configuration file: p2p.trusted_peers\[0\].id: Odd number of digits at line 13 column 11 |-\> p2p.trusted_peers\[0\].id: Odd number of digits at line 13 column 11 [\#1094](https://github.com/input-output-hk/jormungandr/issues/1094)
- Mined transactions are not propagated to other nodes - RC5, local cluster with 2 nodes [\#1090](https://github.com/input-output-hk/jormungandr/issues/1090)
- Transactions not propagating [\#1042](https://github.com/input-output-hk/jormungandr/issues/1042)
- Sync issues not fully resolved [\#1031](https://github.com/input-output-hk/jormungandr/issues/1031)

**Merged pull requests:**

- breaking change: add discriminant account signature and reward account [\#1116](https://github.com/input-output-hk/jormungandr/pull/1116)
- Port Jormungandr to 2018 edition [\#1115](https://github.com/input-output-hk/jormungandr/pull/1115)
- fixed create-account-and-delegate.shtmpl to work with 0.7.0-rc7 [\#1111](https://github.com/input-output-hk/jormungandr/pull/1111)
- Add old utxo and old address to explorer [\#1109](https://github.com/input-output-hk/jormungandr/pull/1109)
- Update Quick-Start in public mode section [\#1108](https://github.com/input-output-hk/jormungandr/pull/1108)
- Simplify Storage::stream_from_to [\#1102](https://github.com/input-output-hk/jormungandr/pull/1102)
- registering_stake_pool.md: do not longer sign the cert [\#1101](https://github.com/input-output-hk/jormungandr/pull/1101)

## [v0.7.0-rc7](https://github.com/input-output-hk/jormungandr/tree/v0.7.0-rc7) (2019-11-08)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.0-rc6...v0.7.0-rc7)

**Implemented enhancements:**

- Logging improvements for network and chain pull [\#1092](https://github.com/input-output-hk/jormungandr/pull/1092)

**Fixed bugs:**

- Fix blocks in epoch first cursor not being 0 [\#1096](https://github.com/input-output-hk/jormungandr/pull/1096)

**Merged pull requests:**

- Revert "Flip the switch on PushHeaders" [\#1095](https://github.com/input-output-hk/jormungandr/pull/1095)

## [v0.7.0-rc6](https://github.com/input-output-hk/jormungandr/tree/v0.7.0-rc6) (2019-11-08)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.0-rc5...v0.7.0-rc6)

**Implemented enhancements:**

- add default custom modules to handle unreachable nodes [\#1091](https://github.com/input-output-hk/jormungandr/pull/1091)

**Closed issues:**

- when is coming out new block for RC5! [\#1087](https://github.com/input-output-hk/jormungandr/issues/1087)
- compilation errors "no method named `pools` found" [\#1085](https://github.com/input-output-hk/jormungandr/issues/1085)
- Transactions are not propagated to the Stake Pool nodes [\#966](https://github.com/input-output-hk/jormungandr/issues/966)

**Merged pull requests:**

- \[Tests\] Jormungandr-scenario-tests node in persistent mode [\#1046](https://github.com/input-output-hk/jormungandr/pull/1046)

## [v0.7.0-rc5](https://github.com/input-output-hk/jormungandr/tree/v0.7.0-rc5) (2019-11-07)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.0-rc4...v0.7.0-rc5)

**Implemented enhancements:**

- UTxO query by FragmentID [\#1048](https://github.com/input-output-hk/jormungandr/issues/1048)
- update chain-deps to have support for ratio delegation [\#1084](https://github.com/input-output-hk/jormungandr/pull/1084)
- Reimplemented chain pull [\#1078](https://github.com/input-output-hk/jormungandr/pull/1078)
- jcli transaction - add data-for-witness \(alias id\) and fragment-id [\#1073](https://github.com/input-output-hk/jormungandr/pull/1073)
- Transaction update \(major breaking changes\) [\#1063](https://github.com/input-output-hk/jormungandr/pull/1063)

**Fixed bugs:**

- Memory allocation error after update to 0.7.0-rc4 [\#1064](https://github.com/input-output-hk/jormungandr/issues/1064)
- Crash on Jormungandr 0.6.0 on Mac OSX Catalina [\#953](https://github.com/input-output-hk/jormungandr/issues/953)
- fix mis-handling of legacy UTxO in the ledger [\#1071](https://github.com/input-output-hk/jormungandr/pull/1071)
- prevent blowing up limit and mitigate generic serialisation format [\#1066](https://github.com/input-output-hk/jormungandr/pull/1066)

**Closed issues:**

- v0.7.0-rc4 Unable to read previously node config [\#1061](https://github.com/input-output-hk/jormungandr/issues/1061)
- Transactions lost on rollbacks [\#1043](https://github.com/input-output-hk/jormungandr/issues/1043)
- Leader node crash during leader event processing on Windows 10 [\#975](https://github.com/input-output-hk/jormungandr/issues/975)

**Merged pull requests:**

- Rest utxo [\#1081](https://github.com/input-output-hk/jormungandr/pull/1081)
- \[Tests\] move NodeState to jormungandr-lib [\#1080](https://github.com/input-output-hk/jormungandr/pull/1080)
- doc: Remove remaining traces of private_id [\#1079](https://github.com/input-output-hk/jormungandr/pull/1079)
- \[Tests \] Stats rest method for node [\#1077](https://github.com/input-output-hk/jormungandr/pull/1077)
- Implement a custom policy object [\#1076](https://github.com/input-output-hk/jormungandr/pull/1076)
- remove unused imports [\#1075](https://github.com/input-output-hk/jormungandr/pull/1075)
- \[Tests\] Use KeyPair from jormungandr-libs [\#1074](https://github.com/input-output-hk/jormungandr/pull/1074)
- Restore REST TX info functionality [\#1070](https://github.com/input-output-hk/jormungandr/pull/1070)
- jcli: certificate print - also signedcert [\#1068](https://github.com/input-output-hk/jormungandr/pull/1068)
- Update introduction.md [\#1062](https://github.com/input-output-hk/jormungandr/pull/1062)
- Docs: allow_private_addresses - add to sample cfg [\#1059](https://github.com/input-output-hk/jormungandr/pull/1059)
- \[Tests\] Testnet test case stability fix [\#1051](https://github.com/input-output-hk/jormungandr/pull/1051)
- \[Tests\] Jormungandr-scenario-tests added grpc calls to node [\#1047](https://github.com/input-output-hk/jormungandr/pull/1047)
- \[Tests\] Genesis initial test cases [\#1023](https://github.com/input-output-hk/jormungandr/pull/1023)

## [v0.7.0-rc4](https://github.com/input-output-hk/jormungandr/tree/v0.7.0-rc4) (2019-11-01)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.0-rc3...v0.7.0-rc4)

**Implemented enhancements:**

- **BREAKING CHANGE**: Lay ground for policy management and update [\#1058](https://github.com/input-output-hk/jormungandr/pull/1058)

**Closed issues:**

- Passive node does not sync \(receive blocks\) - `v0.7.0-rc3` \(local cluster, 2 nodes\) [\#1057](https://github.com/input-output-hk/jormungandr/issues/1057)

**Merged pull requests:**

- Inbound streaming post-fixes [\#1055](https://github.com/input-output-hk/jormungandr/pull/1055)
- \[Tests\] Jomungandr bootstrap fix [\#1050](https://github.com/input-output-hk/jormungandr/pull/1050)

## [v0.7.0-rc3](https://github.com/input-output-hk/jormungandr/tree/v0.7.0-rc3) (2019-10-31)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.0-rc2...v0.7.0-rc3)

**Merged pull requests:**

- Update chain-deps for inbound streaming changes [\#1052](https://github.com/input-output-hk/jormungandr/pull/1052)
- \[Tests\] add error_chain to grpc mock [\#1045](https://github.com/input-output-hk/jormungandr/pull/1045)
- \[Tests\] Recovery new testcase \(automation for \#1011\) [\#1017](https://github.com/input-output-hk/jormungandr/pull/1017)

## [v0.7.0-rc2](https://github.com/input-output-hk/jormungandr/tree/v0.7.0-rc2) (2019-10-29)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.7.0-rc1...v0.7.0-rc2)

**Implemented enhancements:**

- Add old UTXO support to make-witness [\#1036](https://github.com/input-output-hk/jormungandr/pull/1036)

**Closed issues:**

- Failed compiling from source on NixOS [\#1026](https://github.com/input-output-hk/jormungandr/issues/1026)
- Old address transaction witness [\#1005](https://github.com/input-output-hk/jormungandr/issues/1005)

**Merged pull requests:**

- Drill down on connect errors in logs [\#1041](https://github.com/input-output-hk/jormungandr/pull/1041)
- \[Tests\] Testnet testcase fix [\#1032](https://github.com/input-output-hk/jormungandr/pull/1032)
- \[Tests\] Extract trusted peers definition outside script into env var [\#1029](https://github.com/input-output-hk/jormungandr/pull/1029)
- \[Tests\] Jormungandr Grpc mock tests [\#1021](https://github.com/input-output-hk/jormungandr/pull/1021)

## [v0.7.0-rc1](https://github.com/input-output-hk/jormungandr/tree/v0.7.0-rc1) (2019-10-23)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.6.5...v0.7.0-rc1)

**Fixed bugs:**

- Cannot encode genesis.yaml with legacy_funds entry [\#968](https://github.com/input-output-hk/jormungandr/issues/968)
- Update chain-deps [\#1013](https://github.com/input-output-hk/jormungandr/pull/1013)
- Old gossip may cause the node to connect to self [\#1016](https://github.com/input-output-hk/jormungandr/issues/1016)

**Merged pull requests:**

- Safe guard against connecting to self by mistake [\#1019](https://github.com/input-output-hk/jormungandr/pull/1019)
- Logging improvements in the network task [\#1014](https://github.com/input-output-hk/jormungandr/pull/1014)
- \[Tests\] Extract common folder from test configuration [\#914](https://github.com/input-output-hk/jormungandr/pull/914)
- Add node state to REST node stats [\#986](https://github.com/input-output-hk/jormungandr/pull/986)
- Change the start up order for node processes [\#857](https://github.com/input-output-hk/jormungandr/issues/857)
- Breaking Change: header update in performance and security [\#964](https://github.com/input-output-hk/jormungandr/pull/964)
- Start REST server before bootstrap [\#977](https://github.com/input-output-hk/jormungandr/pull/977)

## [v0.6.5](https://github.com/input-output-hk/jormungandr/tree/v0.6.5) (2019-10-19)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.6.4...v0.6.5)

**Fixed bugs:**

- Fix a hangup in network client polling [\#994](https://github.com/input-output-hk/jormungandr/pull/994)

**Closed issues:**

- Node getting stuck [\#993](https://github.com/input-output-hk/jormungandr/issues/993)

## [v0.6.4](https://github.com/input-output-hk/jormungandr/tree/v0.6.4) (2019-10-19)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.6.3...v0.6.4)

**Closed issues:**

- panicking jormungandr 6.3 [\#989](https://github.com/input-output-hk/jormungandr/issues/989)

**Merged pull requests:**

- Fix busy looping in connect [\#992](https://github.com/input-output-hk/jormungandr/pull/992)

## [v0.6.3](https://github.com/input-output-hk/jormungandr/tree/v0.6.3) (2019-10-18)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.6.2...v0.6.3)

**Implemented enhancements:**

- network: Get rid of DefaultExecutor [\#985](https://github.com/input-output-hk/jormungandr/pull/985)
- Add block search by stake pool [\#960](https://github.com/input-output-hk/jormungandr/pull/960)

**Fixed bugs:**

- Update longest chain only after successful insert [\#984](https://github.com/input-output-hk/jormungandr/pull/984)

**Merged pull requests:**

- doc: private_id is optional, used for trusted peers [\#959](https://github.com/input-output-hk/jormungandr/pull/959)

## [v0.6.2](https://github.com/input-output-hk/jormungandr/tree/v0.6.2) (2019-10-18)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.6.1...v0.6.2)

**Implemented enhancements:**

- Add --rest-listen-address Jormungandr CLI arg [\#925](https://github.com/input-output-hk/jormungandr/pull/925)

**Fixed bugs:**

- The existential terror of non existence - a soliloquy. [\#941](https://github.com/input-output-hk/jormungandr/issues/941)

**Closed issues:**

- Not receiving block v0.6.1 [\#973](https://github.com/input-output-hk/jormungandr/issues/973)
- Error while parsing the node configuration file [\#971](https://github.com/input-output-hk/jormungandr/issues/971)
- Trusted peer responded with different node id [\#965](https://github.com/input-output-hk/jormungandr/issues/965)
- Jormungandr 0.6.1 // windows10 // public ip // from binaries : Thread Panicked Error [\#962](https://github.com/input-output-hk/jormungandr/issues/962)

**Merged pull requests:**

- Rework pending client connections [\#981](https://github.com/input-output-hk/jormungandr/pull/981)
- update ContentBuilder/BlockBuilder interface [\#963](https://github.com/input-output-hk/jormungandr/pull/963)

## [v0.6.1](https://github.com/input-output-hk/jormungandr/tree/v0.6.1) (2019-10-15)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.6.0...v0.6.1)

**Implemented enhancements:**

- Accept sensitive parameters for jormungandr via environment variables. [\#935](https://github.com/input-output-hk/jormungandr/issues/935)
- treat the network blocks just like every other blocks even at bootstrap [\#949](https://github.com/input-output-hk/jormungandr/pull/949)

**Fixed bugs:**

- ERRO cannot propagate block to network: send failed because channel is full [\#861](https://github.com/input-output-hk/jormungandr/issues/861)

**Closed issues:**

- Does not compile on Ubuntu 18.04.3 LTS \(rustc 1.36\) [\#948](https://github.com/input-output-hk/jormungandr/issues/948)

## [v0.6.0](https://github.com/input-output-hk/jormungandr/tree/v0.6.0) (2019-10-14)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.5.6...v0.6.0)

**Implemented enhancements:**

- Information and statistics per network peer [\#846](https://github.com/input-output-hk/jormungandr/issues/846)
- The user experience is low after using the single command to start the node [\#825](https://github.com/input-output-hk/jormungandr/issues/825)
- Network stats [\#939](https://github.com/input-output-hk/jormungandr/pull/939)
- Fix and improvements in the fragment logs handling [\#931](https://github.com/input-output-hk/jormungandr/pull/931)
- Add --rest-listen-address Jormungandr CLI arg [\#925](https://github.com/input-output-hk/jormungandr/pull/925)
- Fix REST server panic when get_block_id gets nonexistent block ID [\#912](https://github.com/input-output-hk/jormungandr/pull/912)
- JCLI: transaction info - remove args positionality [\#910](https://github.com/input-output-hk/jormungandr/pull/910)
- Put a hard limit on incoming p2p connections [\#909](https://github.com/input-output-hk/jormungandr/pull/909)

**Fixed bugs:**

- task: leadership thread 'leadership2' panicked \(node continue to run\) [\#930](https://github.com/input-output-hk/jormungandr/issues/930)
- Suspected file descriptor leak \(ERRO Error while accepting connection on 0.0.0.0:3000: Os\) [\#923](https://github.com/input-output-hk/jormungandr/issues/923)
- Querying the node for an inexisting block data, panics! [\#859](https://github.com/input-output-hk/jormungandr/issues/859)
- initial bootstrap completedthread 'network, ' panicked at 'not yet implementedpeer_addr' [\#849](https://github.com/input-output-hk/jormungandr/issues/849)
- leadership module does not purge logs. [\#842](https://github.com/input-output-hk/jormungandr/issues/842)
- Fix and improvements in the fragment logs handling [\#931](https://github.com/input-output-hk/jormungandr/pull/931)
- Fix REST server panic when get\\\_block\\\_id gets nonexistent block ID [\#912](https://github.com/input-output-hk/jormungandr/pull/912)
- Fix end and start cursors in block connection [\#904](https://github.com/input-output-hk/jormungandr/pull/904)

**Closed issues:**

- /api/v0/account/{account_id} truncating address resulting in 404 not found [\#908](https://github.com/input-output-hk/jormungandr/issues/908)

**Merged pull requests:**

- Reuse pending client connections [\#940](https://github.com/input-output-hk/jormungandr/pull/940)
- add PeerStats's creation time [\#938](https://github.com/input-output-hk/jormungandr/pull/938)
- general code improvements [\#937](https://github.com/input-output-hk/jormungandr/pull/937)
- Don't advise build from master but from latest release tag. [\#934](https://github.com/input-output-hk/jormungandr/pull/934)
- dropping \*BSD builds [\#933](https://github.com/input-output-hk/jormungandr/pull/933)
- add cargo audit in our circle ci jobs [\#929](https://github.com/input-output-hk/jormungandr/pull/929)
- update dependencies [\#928](https://github.com/input-output-hk/jormungandr/pull/928)
- Fix node that do not sync when no Public Address is set [\#926](https://github.com/input-output-hk/jormungandr/pull/926)
- Per-peer statistics on items received via subscription channels [\#921](https://github.com/input-output-hk/jormungandr/pull/921)
- Make network task startup fail on listen failure [\#911](https://github.com/input-output-hk/jormungandr/pull/911)
- \[Testnet\] E2E test for stake pool [\#900](https://github.com/input-output-hk/jormungandr/pull/900)

## [v0.5.6](https://github.com/input-output-hk/jormungandr/tree/v0.5.6) (2019-10-07)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.5.5...v0.5.6)

**Implemented enhancements:**

- \(enhancement\) Enhance the output of `rest v0 settings get` command [\#884](https://github.com/input-output-hk/jormungandr/issues/884)
- Fix panic on short-lived incoming connections [\#899](https://github.com/input-output-hk/jormungandr/pull/899)
- Add paginated blocks query as a BlockConnection [\#889](https://github.com/input-output-hk/jormungandr/pull/889)
- Jormungandr: enrich rest get_settings [\#887](https://github.com/input-output-hk/jormungandr/pull/887)
- Add certificate query for transaction [\#878](https://github.com/input-output-hk/jormungandr/pull/878)

**Fixed bugs:**

- Errors reported in logs on 0.5.4 [\#867](https://github.com/input-output-hk/jormungandr/issues/867)
- Return HTTP 400 from next_id when block is not in tip chain [\#894](https://github.com/input-output-hk/jormungandr/pull/894)

**Closed issues:**

- 500 Internal Server error querying next-id [\#888](https://github.com/input-output-hk/jormungandr/issues/888)
- Connection refused \(os error 111\) [\#886](https://github.com/input-output-hk/jormungandr/issues/886)
- My local node/stake pool did not create any block [\#840](https://github.com/input-output-hk/jormungandr/issues/840)

**Merged pull requests:**

- Handle listening error in run_listen_socket [\#901](https://github.com/input-output-hk/jormungandr/pull/901)
- bump our full version generator library to include the proper target ARCH and OS [\#898](https://github.com/input-output-hk/jormungandr/pull/898)
- Ignore per-connection accept\(\) errors [\#896](https://github.com/input-output-hk/jormungandr/pull/896)
- Log termination of P2P connections [\#892](https://github.com/input-output-hk/jormungandr/pull/892)
- Network improvements: less noisy logging, evict peers more actively [\#891](https://github.com/input-output-hk/jormungandr/pull/891)
- Implement connection limit [\#890](https://github.com/input-output-hk/jormungandr/pull/890)
- Less spammy network logging [\#885](https://github.com/input-output-hk/jormungandr/pull/885)

## [v0.5.5](https://github.com/input-output-hk/jormungandr/tree/v0.5.5) (2019-10-01)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.5.4...v0.5.5)

**Implemented enhancements:**

- add more logging information and comfort for users [\#880](https://github.com/input-output-hk/jormungandr/pull/880)
- be more liberal with the node's resources and trust default values [\#874](https://github.com/input-output-hk/jormungandr/pull/874)
- stop the node if we detect a panic in a runtime [\#873](https://github.com/input-output-hk/jormungandr/pull/873)

**Fixed bugs:**

- Panic in ReplyStreamHandle methods when the receiver stream is closed by the client [\#864](https://github.com/input-output-hk/jormungandr/issues/864)
- Panic at Result::unwrap\(\) [\#850](https://github.com/input-output-hk/jormungandr/issues/850)
- Don't panic when intercom reply is cancelled [\#876](https://github.com/input-output-hk/jormungandr/pull/876)

**Closed issues:**

- Compiling jormungandr builds two versions of SHA2 libraries [\#875](https://github.com/input-output-hk/jormungandr/issues/875)

**Merged pull requests:**

- Update chain-deps and poldercast [\#877](https://github.com/input-output-hk/jormungandr/pull/877)
- Eliminate a panic on block task message box [\#870](https://github.com/input-output-hk/jormungandr/pull/870)

## [v0.5.4](https://github.com/input-output-hk/jormungandr/tree/v0.5.4) (2019-09-30)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.5.3...v0.5.4)

**Fixed bugs:**

- Crash panicked at internal error entered unreachable code [\#865](https://github.com/input-output-hk/jormungandr/issues/865)
- update poldercast to 0.7.1 [\#866](https://github.com/input-output-hk/jormungandr/pull/866)

## [v0.5.3](https://github.com/input-output-hk/jormungandr/tree/v0.5.3) (2019-09-30)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.5.2...v0.5.3)

**Implemented enhancements:**

- accept host for jcli rest via env var [\#854](https://github.com/input-output-hk/jormungandr/issues/854)
- Evict p2p peers after a failed client connection [\#862](https://github.com/input-output-hk/jormungandr/pull/862)
- don't use all the trusted peers unless it is actually necessary [\#848](https://github.com/input-output-hk/jormungandr/pull/848)

**Fixed bugs:**

- Valgrind additional diagnostics during connection to cluster [\#833](https://github.com/input-output-hk/jormungandr/issues/833)
- Node crash with "free\(\): invalid pointer" [\#819](https://github.com/input-output-hk/jormungandr/issues/819)

**Closed issues:**

- check address balance - rest request error [\#856](https://github.com/input-output-hk/jormungandr/issues/856)
- Error downloading initial bootstrap from official trusted peers [\#843](https://github.com/input-output-hk/jormungandr/issues/843)
- Unwrap fails in network::grpc::client::connect [\#803](https://github.com/input-output-hk/jormungandr/issues/803)

**Merged pull requests:**

- JCLI: accept rest host via env var [\#858](https://github.com/input-output-hk/jormungandr/pull/858)
- some cleanup for future releases [\#847](https://github.com/input-output-hk/jormungandr/pull/847)
- Lower bootstrap error log level to warnings [\#845](https://github.com/input-output-hk/jormungandr/pull/845)
- Add quickstart explorer documentation page [\#809](https://github.com/input-output-hk/jormungandr/pull/809)

## [v0.5.2](https://github.com/input-output-hk/jormungandr/tree/v0.5.2) (2019-09-26)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.5.1...v0.5.2)

**Implemented enhancements:**

- JCLI: management_threshold validity [\#838](https://github.com/input-output-hk/jormungandr/pull/838)

**Fixed bugs:**

- Fix peer map pointer update bug [\#841](https://github.com/input-output-hk/jormungandr/pull/841)

**Closed issues:**

- improve jcli certificate new-stake-pool-certificate error handling [\#837](https://github.com/input-output-hk/jormungandr/issues/837)

## [v0.5.1](https://github.com/input-output-hk/jormungandr/tree/v0.5.1) (2019-09-26)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.5.0...v0.5.1)

**Implemented enhancements:**

- Try trusted peers in random order [\#831](https://github.com/input-output-hk/jormungandr/pull/831)
- general cleanup and display better error messages [\#830](https://github.com/input-output-hk/jormungandr/pull/830)
- Filter private addresses from gossip [\#810](https://github.com/input-output-hk/jormungandr/pull/810)
- Use imhamt and multiverse in explorer [\#771](https://github.com/input-output-hk/jormungandr/pull/771)

**Fixed bugs:**

- `lastBlockTime` parameter \(for jcli rest v0 node stats\) does not return any value [\#834](https://github.com/input-output-hk/jormungandr/issues/834)
- The node will not start if the first trusted-peer in the list is not reachable [\#824](https://github.com/input-output-hk/jormungandr/issues/824)
- Node crash with "free\(\): invalid next size \(fast\)" [\#820](https://github.com/input-output-hk/jormungandr/issues/820)
- Thread panicked/PoisonError while running local node connected to Nicolas's trusted-peer [\#818](https://github.com/input-output-hk/jormungandr/issues/818)
- Error when fetching blocks from a peer [\#816](https://github.com/input-output-hk/jormungandr/issues/816)

**Closed issues:**

- Transaction for sending the stake-pool certificate is rejected with `Pool Registration certificate invalid` [\#836](https://github.com/input-output-hk/jormungandr/issues/836)
- To many ConnectError messages in the logs for the same unreachable node [\#828](https://github.com/input-output-hk/jormungandr/issues/828)
- IPv6 local nodes does not connect to IPv4 trusted peers [\#826](https://github.com/input-output-hk/jormungandr/issues/826)
- Error while starting the node with the single `jormungandr` command [\#821](https://github.com/input-output-hk/jormungandr/issues/821)
- Filter addresses that are not reachable [\#799](https://github.com/input-output-hk/jormungandr/issues/799)

**Merged pull requests:**

- set the slot start time to the correct value [\#835](https://github.com/input-output-hk/jormungandr/pull/835)
- Make bump_peer_for_block_fetch miss non-fatal [\#811](https://github.com/input-output-hk/jormungandr/pull/811)
- reduce the error level when peers have invalid addresses [\#807](https://github.com/input-output-hk/jormungandr/pull/807)

## [v0.5.0](https://github.com/input-output-hk/jormungandr/tree/v0.5.0) (2019-09-23)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.3.9999...v0.5.0)

**Implemented enhancements:**

- Unreachable nodes should not be mentioned in gossiping protocol [\#796](https://github.com/input-output-hk/jormungandr/issues/796)
- Allow non reachable nodes, use actual interests and fix in mempool's logs [\#800](https://github.com/input-output-hk/jormungandr/pull/800)
- Remove fragments added to block from fragment pool V1 [\#798](https://github.com/input-output-hk/jormungandr/pull/798)

**Fixed bugs:**

- Fragment propagation on the network fails [\#742](https://github.com/input-output-hk/jormungandr/issues/742)

**Merged pull requests:**

- Get the trusted peer list from the CLI and make `--config` optional [\#805](https://github.com/input-output-hk/jormungandr/pull/805)
- Fix URI formatting for IPv6 addresses [\#804](https://github.com/input-output-hk/jormungandr/pull/804)
- change jormungandr to integrate with the incentive changes [\#802](https://github.com/input-output-hk/jormungandr/pull/802)
- only update the block counter once the block has been validated [\#801](https://github.com/input-output-hk/jormungandr/pull/801)

## [v0.3.9999](https://github.com/input-output-hk/jormungandr/tree/v0.3.9999) (2019-09-20)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.3.1415...v0.3.9999)

**Fixed bugs:**

- fix silly bug in the checkpoint [\#794](https://github.com/input-output-hk/jormungandr/pull/794)

**Merged pull requests:**

- Logging fixes [\#793](https://github.com/input-output-hk/jormungandr/pull/793)

## [v0.3.1415](https://github.com/input-output-hk/jormungandr/tree/v0.3.1415) (2019-09-18)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.3.3...v0.3.1415)

**Implemented enhancements:**

- Jormungandr configuration does not check unknown fields [\#759](https://github.com/input-output-hk/jormungandr/issues/759)
- Extended info on --version [\#719](https://github.com/input-output-hk/jormungandr/issues/719)
- Link network fragment subscription with fragment pool [\#784](https://github.com/input-output-hk/jormungandr/pull/784)
- make sure we keep up to date the right branches on the different network [\#773](https://github.com/input-output-hk/jormungandr/pull/773)
- Fragment validator [\#748](https://github.com/input-output-hk/jormungandr/pull/748)
- Outline expected schema [\#739](https://github.com/input-output-hk/jormungandr/pull/739)
- Versioning improvements for jcli and jörmungandr [\#730](https://github.com/input-output-hk/jormungandr/pull/730)
- Add explorer mode startup config [\#702](https://github.com/input-output-hk/jormungandr/pull/702)

**Fixed bugs:**

- not yet implemented: method to load a Ref from the storage is not yet there [\#788](https://github.com/input-output-hk/jormungandr/issues/788)
- Chain head storage tag not kept up to date [\#783](https://github.com/input-output-hk/jormungandr/issues/783)
- Jormungandr configuration does not check unknown fields [\#759](https://github.com/input-output-hk/jormungandr/issues/759)
- 'block subscription stream failure' when starting a Passive node connected to a Leader node [\#754](https://github.com/input-output-hk/jormungandr/issues/754)
- Network error: Tree topology, PullBlocksToTip issue [\#745](https://github.com/input-output-hk/jormungandr/issues/745)
- `--full-version` and `--source-version` fail since `--config \<node\_config\>` is mandatory [\#732](https://github.com/input-output-hk/jormungandr/issues/732)
- Make sure the TIP's tag is updated in the storage too [\#790](https://github.com/input-output-hk/jormungandr/pull/790)
- add missing break in the bootstrap function [\#753](https://github.com/input-output-hk/jormungandr/pull/753)

**Closed issues:**

- Leader node stops creating blocks [\#776](https://github.com/input-output-hk/jormungandr/issues/776)
- Server Error when directing rest calls to public_address instead of rest address [\#775](https://github.com/input-output-hk/jormungandr/issues/775)

**Merged pull requests:**

- Improve processing of inbound subscription streams [\#789](https://github.com/input-output-hk/jormungandr/pull/789)
- network: Replace forward combinator with send_all [\#787](https://github.com/input-output-hk/jormungandr/pull/787)
- Update chain-deps: Rename content to fragment [\#786](https://github.com/input-output-hk/jormungandr/pull/786)
- Update chain-deps for future-to-sink network API [\#782](https://github.com/input-output-hk/jormungandr/pull/782)
- Rename FragmentSubscription to ContentSubscription in network docs [\#781](https://github.com/input-output-hk/jormungandr/pull/781)
- Update chain-deps; fuse forwarded streams [\#780](https://github.com/input-output-hk/jormungandr/pull/780)
- add network informations [\#779](https://github.com/input-output-hk/jormungandr/pull/779)
- Plug in logging through log crate to slog [\#778](https://github.com/input-output-hk/jormungandr/pull/778)
- Less scary connection error logging [\#777](https://github.com/input-output-hk/jormungandr/pull/777)
- Fall back to block 0 when no starting checkpoints match [\#772](https://github.com/input-output-hk/jormungandr/pull/772)
- remove pre-jormungandr configs [\#770](https://github.com/input-output-hk/jormungandr/pull/770)
- provide better checkpoints than before [\#767](https://github.com/input-output-hk/jormungandr/pull/767)
- Small update to improve perf and memory usage of blockchain cache [\#764](https://github.com/input-output-hk/jormungandr/pull/764)
- config cleanup and validation [\#762](https://github.com/input-output-hk/jormungandr/pull/762)
- Restore logs check [\#761](https://github.com/input-output-hk/jormungandr/pull/761)
- \[Documentation\] Remove public_id from docs [\#758](https://github.com/input-output-hk/jormungandr/pull/758)
- Remove Id from the poldercast gossiping [\#757](https://github.com/input-output-hk/jormungandr/pull/757)
- Offer content service on the server Node [\#755](https://github.com/input-output-hk/jormungandr/pull/755)
- explicit compilation of the node and jcli [\#751](https://github.com/input-output-hk/jormungandr/pull/751)
- make sure we don't build the integration tests dependencies unless needed [\#750](https://github.com/input-output-hk/jormungandr/pull/750)
- go through the whole list of trusted peers on the network [\#749](https://github.com/input-output-hk/jormungandr/pull/749)
- Process fragment subscription on the server side [\#747](https://github.com/input-output-hk/jormungandr/pull/747)
- Add stub to process fragment subscription [\#743](https://github.com/input-output-hk/jormungandr/pull/743)
- Update chain-deps, use ContentService to subscribe the client to fragments [\#740](https://github.com/input-output-hk/jormungandr/pull/740)
- Fragment process clean up [\#737](https://github.com/input-output-hk/jormungandr/pull/737)
- make the node use multiaddr for the listen_addr [\#736](https://github.com/input-output-hk/jormungandr/pull/736)
- Clean up fragment network API docs [\#735](https://github.com/input-output-hk/jormungandr/pull/735)
- Update to upload sinks in network-core API [\#734](https://github.com/input-output-hk/jormungandr/pull/734)
- fix issue with --full-version expecting the --config [\#733](https://github.com/input-output-hk/jormungandr/pull/733)
- Openapi lvl4 [\#731](https://github.com/input-output-hk/jormungandr/pull/731)
- Update stats and scenario testing [\#729](https://github.com/input-output-hk/jormungandr/pull/729)
- Openapi validator lvl3 [\#728](https://github.com/input-output-hk/jormungandr/pull/728)
- Add graphql server [\#727](https://github.com/input-output-hk/jormungandr/pull/727)
- Cert update [\#726](https://github.com/input-output-hk/jormungandr/pull/726)
- Openapi verifier lvl2 [\#725](https://github.com/input-output-hk/jormungandr/pull/725)
- JCLI: add rest/v0/stake [\#722](https://github.com/input-output-hk/jormungandr/pull/722)
- doc: renaming genesis to genesis_praos [\#721](https://github.com/input-output-hk/jormungandr/pull/721)
- capture the standard error output from the running nodes [\#718](https://github.com/input-output-hk/jormungandr/pull/718)

## [v0.3.3](https://github.com/input-output-hk/jormungandr/tree/v0.3.3) (2019-08-22)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.3.2...v0.3.3)

**Implemented enhancements:**

- CORS not enabled [\#708](https://github.com/input-output-hk/jormungandr/issues/708)
- add some logging on the leader event created block [\#717](https://github.com/input-output-hk/jormungandr/pull/717)
- Add CORS to REST endpoint [\#711](https://github.com/input-output-hk/jormungandr/pull/711)
- Add leadership logs fetching over REST API and JCLI [\#707](https://github.com/input-output-hk/jormungandr/pull/707)
- Finalize the loop for the new leadership code/event [\#701](https://github.com/input-output-hk/jormungandr/pull/701)
- allow setting the address prefix manually when creating the address [\#695](https://github.com/input-output-hk/jormungandr/pull/695)
- blockchain task: Process block announcements and blocks from network [\#693](https://github.com/input-output-hk/jormungandr/pull/693)
- plumbing changes for the leadership scheduling and blockchain validation [\#688](https://github.com/input-output-hk/jormungandr/pull/688)
- Update Leadership module to handle new blockchain API [\#685](https://github.com/input-output-hk/jormungandr/pull/685)
- Added block processing for new blockchain [\#684](https://github.com/input-output-hk/jormungandr/pull/684)

**Fixed bugs:**

- \[Jormungandr\] - \[mempool\] : Node "stops" producing blocks if garbage_collection_interval \< fragment_ttl [\#705](https://github.com/input-output-hk/jormungandr/issues/705)
- Database error after abrupt node restart [\#676](https://github.com/input-output-hk/jormungandr/issues/676)
- make sure we don't block the poll in the fragment pool too [\#706](https://github.com/input-output-hk/jormungandr/pull/706)
- Mempool and Leadership logs GC setting and fixes [\#703](https://github.com/input-output-hk/jormungandr/pull/703)
- Fix tests aborting on invalid logs [\#689](https://github.com/input-output-hk/jormungandr/pull/689)
- Added block processing for new blockchain [\#684](https://github.com/input-output-hk/jormungandr/pull/684)

**Closed issues:**

- serve the leader logs through the Rest API [\#698](https://github.com/input-output-hk/jormungandr/issues/698)

**Merged pull requests:**

- Testing scenario managing test flow [\#716](https://github.com/input-output-hk/jormungandr/pull/716)
- Testing scenario managing test flow [\#715](https://github.com/input-output-hk/jormungandr/pull/715)
- Rest async [\#714](https://github.com/input-output-hk/jormungandr/pull/714)
- \[Tests\] Fixed test_genesis_stake_pool_with_utxo_faucet_starts_successfully [\#713](https://github.com/input-output-hk/jormungandr/pull/713)
- Minor improvements on scenario testing [\#712](https://github.com/input-output-hk/jormungandr/pull/712)
- experiment with new interface for multi nodes testing [\#710](https://github.com/input-output-hk/jormungandr/pull/710)
- Futures rest [\#700](https://github.com/input-output-hk/jormungandr/pull/700)
- Deps cleanup [\#699](https://github.com/input-output-hk/jormungandr/pull/699)
- add link to the Node REST Api documentation [\#697](https://github.com/input-output-hk/jormungandr/pull/697)
- blockchain: Restore block propagation [\#696](https://github.com/input-output-hk/jormungandr/pull/696)
- Openapi validator 1st level [\#692](https://github.com/input-output-hk/jormungandr/pull/692)
- Use mainstream implementation for hex [\#691](https://github.com/input-output-hk/jormungandr/pull/691)
- Create OpenAPI docs [\#690](https://github.com/input-output-hk/jormungandr/pull/690)
- cosmetic-summing fixes [\#686](https://github.com/input-output-hk/jormungandr/pull/686)
- Update ROADMAP.md [\#669](https://github.com/input-output-hk/jormungandr/pull/669)

## [v0.3.2](https://github.com/input-output-hk/jormungandr/tree/v0.3.2) (2019-08-07)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.3.1...v0.3.2)

**Implemented enhancements:**

- version info in startup messages [\#606](https://github.com/input-output-hk/jormungandr/issues/606)
- Improve naming in config YAML [\#575](https://github.com/input-output-hk/jormungandr/issues/575)
- Extend logs with app version, epoch and slot_id [\#679](https://github.com/input-output-hk/jormungandr/pull/679)
- Graceful handling of block0 in the future [\#661](https://github.com/input-output-hk/jormungandr/pull/661)
- Add stake pool getter to REST API [\#660](https://github.com/input-output-hk/jormungandr/pull/660)
- network: Perform protocol handshake [\#657](https://github.com/input-output-hk/jormungandr/pull/657)
- Add leadership management REST API [\#654](https://github.com/input-output-hk/jormungandr/pull/654)

**Fixed bugs:**

- Upgrade custom_error to 1.7.1 [\#678](https://github.com/input-output-hk/jormungandr/pull/678)
- it seems that debug_assertions feature was not doing what I expected [\#677](https://github.com/input-output-hk/jormungandr/pull/677)
- Graceful handling of block0 in the future [\#661](https://github.com/input-output-hk/jormungandr/pull/661)
- Poll gRPC client ready before sending any requests [\#656](https://github.com/input-output-hk/jormungandr/pull/656)
- Don't let one client connection terminate task [\#650](https://github.com/input-output-hk/jormungandr/pull/650)

**Closed issues:**

- Jcli: address info - wrong subcommand description [\#670](https://github.com/input-output-hk/jormungandr/issues/670)
- jormungandr install error [\#665](https://github.com/input-output-hk/jormungandr/issues/665)
- Jcli: cargo install failure due to custom_error/1.7.1 crate [\#664](https://github.com/input-output-hk/jormungandr/issues/664)
- v0.3.1 Cannot Compile [\#648](https://github.com/input-output-hk/jormungandr/issues/648)
- Add fields useful for logs [\#645](https://github.com/input-output-hk/jormungandr/issues/645)

**Merged pull requests:**

- Remove Mjolnir [\#683](https://github.com/input-output-hk/jormungandr/pull/683)
- Remove unused deps from Jormungandr [\#682](https://github.com/input-output-hk/jormungandr/pull/682)
- internal code design: simple case state machine [\#681](https://github.com/input-output-hk/jormungandr/pull/681)
- Implement bootstrap for new blockchain API [\#675](https://github.com/input-output-hk/jormungandr/pull/675)
- New blockchain data representation [\#673](https://github.com/input-output-hk/jormungandr/pull/673)
- Jcli: Fix - address info, wrong subcommand description [\#671](https://github.com/input-output-hk/jormungandr/pull/671)
- \[Tests\] Set rust backtrace in e2e tests [\#668](https://github.com/input-output-hk/jormungandr/pull/668)
- add ROADMAP [\#666](https://github.com/input-output-hk/jormungandr/pull/666)
- Boxing problem with custom_error [\#662](https://github.com/input-output-hk/jormungandr/pull/662)
- Network fixes [\#655](https://github.com/input-output-hk/jormungandr/pull/655)
- Protocol doc update [\#653](https://github.com/input-output-hk/jormungandr/pull/653)
- Fixed test_correct_utxo_transaction_replaces_old_utxo_by_node test [\#651](https://github.com/input-output-hk/jormungandr/pull/651)
- \[Tests\] Use fragment Id to track transaction status after post [\#647](https://github.com/input-output-hk/jormungandr/pull/647)
- update to latest chain-deps [\#646](https://github.com/input-output-hk/jormungandr/pull/646)
- Push/pull chain as complete blocks in one go [\#639](https://github.com/input-output-hk/jormungandr/pull/639)
- CircleCI: Streamline the workflow [\#625](https://github.com/input-output-hk/jormungandr/pull/625)

## [v0.3.1](https://github.com/input-output-hk/jormungandr/tree/v0.3.1) (2019-07-19)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.3.0...v0.3.1)

**Implemented enhancements:**

- Add node shutdown over REST [\#643](https://github.com/input-output-hk/jormungandr/pull/643)
- Add lastBlock info to node state REST [\#642](https://github.com/input-output-hk/jormungandr/pull/642)
- Add REST endpoint for getting node settings [\#634](https://github.com/input-output-hk/jormungandr/pull/634)

**Closed issues:**

- Node crash with Crit error when sending multiple transactions from the same Account, with the same Counter, in 2 consecutive slots [\#641](https://github.com/input-output-hk/jormungandr/issues/641)
- Transaction rejected because "Account with invalid signature" when sending multiple transactions from the same Account in the same slot \(with different Counter values\) [\#640](https://github.com/input-output-hk/jormungandr/issues/640)

**Merged pull requests:**

- Upgrade Slog to 2.5.1 [\#637](https://github.com/input-output-hk/jormungandr/pull/637)
- Simplify slot_start_time storage to seconds [\#636](https://github.com/input-output-hk/jormungandr/pull/636)
- Remove unused JCLI deps [\#635](https://github.com/input-output-hk/jormungandr/pull/635)
- Rename more fields in p2p config [\#632](https://github.com/input-output-hk/jormungandr/pull/632)

## [v0.3.0](https://github.com/input-output-hk/jormungandr/tree/v0.3.0) (2019-07-12)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.2.4...v0.3.0)

**Implemented enhancements:**

- Log Level consistency between CLI and config file [\#622](https://github.com/input-output-hk/jormungandr/issues/622)
- breaking change: move to fragment id to refer to utxo [\#633](https://github.com/input-output-hk/jormungandr/pull/633)
- Rework handling of inbound blocks and headers from the network [\#626](https://github.com/input-output-hk/jormungandr/pull/626)

**Fixed bugs:**

- Node crash if sending multiple transactions in the same slot [\#586](https://github.com/input-output-hk/jormungandr/issues/586)
- broken link, registering stake key guide [\#565](https://github.com/input-output-hk/jormungandr/issues/565)

**Closed issues:**

- add-output example missing value [\#628](https://github.com/input-output-hk/jormungandr/issues/628)
- gelf logging broken [\#621](https://github.com/input-output-hk/jormungandr/issues/621)
- Transactions are rejected when genesis file is re-encoded manually [\#610](https://github.com/input-output-hk/jormungandr/issues/610)

**Merged pull requests:**

- Rename Message to Fragment [\#631](https://github.com/input-output-hk/jormungandr/pull/631)
- Clean up unnecessary lifetimes in configuration_builder test tools [\#630](https://github.com/input-output-hk/jormungandr/pull/630)
- Small doc updates [\#629](https://github.com/input-output-hk/jormungandr/pull/629)
- Clean up and extend log configuration [\#627](https://github.com/input-output-hk/jormungandr/pull/627)
- Process events in the client connection [\#620](https://github.com/input-output-hk/jormungandr/pull/620)
- Fix node crashing when multiple TXs for same account are in slot [\#619](https://github.com/input-output-hk/jormungandr/pull/619)
- make sure we use the latest stable available [\#616](https://github.com/input-output-hk/jormungandr/pull/616)
- move the address prefix to -lib and jcli [\#614](https://github.com/input-output-hk/jormungandr/pull/614)
- Fix localhost for BSD & OSX [\#613](https://github.com/input-output-hk/jormungandr/pull/613)
- scripts: bootstrap small fixes [\#611](https://github.com/input-output-hk/jormungandr/pull/611)
- \[Test\] node stops producing blocks test case [\#609](https://github.com/input-output-hk/jormungandr/pull/609)
- scripts: create-account-and-delegate POSIX-syntax [\#608](https://github.com/input-output-hk/jormungandr/pull/608)
- jormungandr: fix binary version release 0.2.3 -\> 0.2.4 [\#607](https://github.com/input-output-hk/jormungandr/pull/607)
- Add proper error reporting to JCLI REST commands [\#605](https://github.com/input-output-hk/jormungandr/pull/605)
- Pulling missing chain blocks from the network [\#601](https://github.com/input-output-hk/jormungandr/pull/601)
- use binaries by default and support building source too [\#594](https://github.com/input-output-hk/jormungandr/pull/594)
- Docker: use alpine base image and versioned releases [\#567](https://github.com/input-output-hk/jormungandr/pull/567)

## [v0.2.4](https://github.com/input-output-hk/jormungandr/tree/v0.2.4) (2019-07-04)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.2.3...v0.2.4)

**Implemented enhancements:**

- Improve bootstrap script to prevent a non-stake case to appear [\#604](https://github.com/input-output-hk/jormungandr/pull/604)
- Rest stake distribution [\#603](https://github.com/input-output-hk/jormungandr/pull/603)
- More graceful error handling in blockchain task [\#588](https://github.com/input-output-hk/jormungandr/pull/588)
- More logging improvements; add output to stdout [\#587](https://github.com/input-output-hk/jormungandr/pull/587)

**Fixed bugs:**

- block0 initial funds should accept multiple entries [\#579](https://github.com/input-output-hk/jormungandr/issues/579)
- jcli add-certificate does not take fees into account [\#499](https://github.com/input-output-hk/jormungandr/issues/499)

**Closed issues:**

- bootstrap script error [\#602](https://github.com/input-output-hk/jormungandr/issues/602)
- v0.2.3 Cannot Compile \(Experimental Alpine Docker\) [\#590](https://github.com/input-output-hk/jormungandr/issues/590)
- cargo install compile fail [\#581](https://github.com/input-output-hk/jormungandr/issues/581)
- add-output results in invalid internal encoding error [\#577](https://github.com/input-output-hk/jormungandr/issues/577)
- Documentation : empty faucet warning \(?\) [\#564](https://github.com/input-output-hk/jormungandr/issues/564)
- bootstrap error in genesis_praos, genesis file corrupted [\#562](https://github.com/input-output-hk/jormungandr/issues/562)
- Documentation: Improve the documentation related to Staking&Delegation [\#530](https://github.com/input-output-hk/jormungandr/issues/530)
- documentation: Add a consolidated/consistent/easier way for starting the node [\#515](https://github.com/input-output-hk/jormungandr/issues/515)
- documentation: improve the documentation for 'jcli rest v0 account get' [\#484](https://github.com/input-output-hk/jormungandr/issues/484)

**Merged pull requests:**

- Finalize Divide and Reuse [\#600](https://github.com/input-output-hk/jormungandr/pull/600)
- More changes in the jormungandr-lib API [\#599](https://github.com/input-output-hk/jormungandr/pull/599)
- take into account the certificate when computing the fees [\#598](https://github.com/input-output-hk/jormungandr/pull/598)
- Use certificate from jormungandr lib [\#597](https://github.com/input-output-hk/jormungandr/pull/597)
- Improve testing of the Block0Configuration [\#596](https://github.com/input-output-hk/jormungandr/pull/596)
- updated delegation script [\#595](https://github.com/input-output-hk/jormungandr/pull/595)
- REST refactoring and simplification [\#593](https://github.com/input-output-hk/jormungandr/pull/593)
- Test Improvement. Implement dumping logs on console on jormungandr error [\#592](https://github.com/input-output-hk/jormungandr/pull/592)
- Update chain-deps [\#585](https://github.com/input-output-hk/jormungandr/pull/585)
- experiment with changelog generation [\#584](https://github.com/input-output-hk/jormungandr/pull/584)
- Fix multi output funds support in genesis yaml file [\#583](https://github.com/input-output-hk/jormungandr/pull/583)
- Add notes on protobuf and C compilers to the install steps [\#582](https://github.com/input-output-hk/jormungandr/pull/582)
- Improve GELF logging backend configuration [\#574](https://github.com/input-output-hk/jormungandr/pull/574)
- bootstrap script ported to Windows Powershell [\#572](https://github.com/input-output-hk/jormungandr/pull/572)
- Update blockchain.md [\#568](https://github.com/input-output-hk/jormungandr/pull/568)
- Pull missing blocks [\#554](https://github.com/input-output-hk/jormungandr/pull/554)
- Add example for account input and links to scripts [\#487](https://github.com/input-output-hk/jormungandr/pull/487)

## [v0.2.3](https://github.com/input-output-hk/jormungandr/tree/v0.2.3) (2019-06-23)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.2.2...v0.2.3)

**Merged pull requests:**

- Move JCLI's genesis into jormungandr-lib [\#560](https://github.com/input-output-hk/jormungandr/pull/560)
- Proposal to replace ENTRYPOINT with CMD in Dockerfile [\#559](https://github.com/input-output-hk/jormungandr/pull/559)

## [v0.2.2](https://github.com/input-output-hk/jormungandr/tree/v0.2.2) (2019-06-21)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.2.1...v0.2.2)

**Closed issues:**

- jcli 0.2.1 \[jcli key generate --type\] [\#501](https://github.com/input-output-hk/jormungandr/issues/501)
- REST account API: The delegation field format should be improved [\#491](https://github.com/input-output-hk/jormungandr/issues/491)
- gelf logging support for slog [\#447](https://github.com/input-output-hk/jormungandr/issues/447)

**Merged pull requests:**

- mark gelf as optional feature [\#557](https://github.com/input-output-hk/jormungandr/pull/557)
- Fix incorrect PATH setting [\#555](https://github.com/input-output-hk/jormungandr/pull/555)
- Update introduction.md [\#552](https://github.com/input-output-hk/jormungandr/pull/552)
- Update delegating_stake.md [\#550](https://github.com/input-output-hk/jormungandr/pull/550)
- add more documentation [\#547](https://github.com/input-output-hk/jormungandr/pull/547)
- UTxO Info as a common interface between jcli, jormungandr and the tests [\#543](https://github.com/input-output-hk/jormungandr/pull/543)
- move to chain-deps [\#542](https://github.com/input-output-hk/jormungandr/pull/542)
- ignore unstable test [\#539](https://github.com/input-output-hk/jormungandr/pull/539)
- remove non needed reference [\#538](https://github.com/input-output-hk/jormungandr/pull/538)
- fix the path to the default genesis block in the documentation [\#537](https://github.com/input-output-hk/jormungandr/pull/537)
- add hex-crate to replace cardano::util::hex [\#534](https://github.com/input-output-hk/jormungandr/pull/534)
- account state for both jormungandr, jcli and tests [\#532](https://github.com/input-output-hk/jormungandr/pull/532)
- Revert "New corner cases for transaction module" [\#531](https://github.com/input-output-hk/jormungandr/pull/531)
- Add script for create account and delegating with it [\#529](https://github.com/input-output-hk/jormungandr/pull/529)
- provide more details on the error if available [\#528](https://github.com/input-output-hk/jormungandr/pull/528)
- Move genesis.yaml initial state to single list [\#527](https://github.com/input-output-hk/jormungandr/pull/527)
- Fix for soak test after \#522 [\#526](https://github.com/input-output-hk/jormungandr/pull/526)
- Documentation update [\#524](https://github.com/input-output-hk/jormungandr/pull/524)
- Unify jormungandr API Types to allow better reusability [\#522](https://github.com/input-output-hk/jormungandr/pull/522)
- Update introduction.md [\#521](https://github.com/input-output-hk/jormungandr/pull/521)
- bootstrap: fix printed example command [\#519](https://github.com/input-output-hk/jormungandr/pull/519)
- Changing function declaration to POSIX-syntax [\#518](https://github.com/input-output-hk/jormungandr/pull/518)
- Increase logger async buffer from 128 to 1024 entries [\#509](https://github.com/input-output-hk/jormungandr/pull/509)
- removing dup getopts d [\#504](https://github.com/input-output-hk/jormungandr/pull/504)
- jcli: fix key type in help message ed25510bip32 -\> ed25519bip32 [\#502](https://github.com/input-output-hk/jormungandr/pull/502)
- adding a few more flags/options to the bootstrap script [\#498](https://github.com/input-output-hk/jormungandr/pull/498)
- Support GELF logging output. [\#497](https://github.com/input-output-hk/jormungandr/pull/497)
- Remove todo in quickstart section about P2P [\#486](https://github.com/input-output-hk/jormungandr/pull/486)
- Test stability fix for transaction test cases [\#483](https://github.com/input-output-hk/jormungandr/pull/483)

## [v0.2.1](https://github.com/input-output-hk/jormungandr/tree/v0.2.1) (2019-06-15)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.2.0...v0.2.1)

**Fixed bugs:**

- output and format of logger defined in config yaml is ignored [\#494](https://github.com/input-output-hk/jormungandr/issues/494)
- jcli transaction id not changing when adding certificate [\#475](https://github.com/input-output-hk/jormungandr/issues/475)

**Merged pull requests:**

- JCLI fixes and Ed25519 sk unification [\#500](https://github.com/input-output-hk/jormungandr/pull/500)
- Stop ignoring config.yaml logger settings [\#495](https://github.com/input-output-hk/jormungandr/pull/495)
- Extend faucet script [\#492](https://github.com/input-output-hk/jormungandr/pull/492)
- Poll the gRPC client for readiness [\#489](https://github.com/input-output-hk/jormungandr/pull/489)
- replace invalid TransactionId [\#488](https://github.com/input-output-hk/jormungandr/pull/488)
- Fix README typo: public_access-\>public_address [\#482](https://github.com/input-output-hk/jormungandr/pull/482)
- add option to disable colours, fix find for deleting tmp files [\#480](https://github.com/input-output-hk/jormungandr/pull/480)
- Stake key certificate does not exist anymore [\#461](https://github.com/input-output-hk/jormungandr/pull/461)

## [v0.2.0](https://github.com/input-output-hk/jormungandr/tree/v0.2.0) (2019-06-13)

[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.1.0...v0.2.0)

**Fixed bugs:**

- Error when verifying transaction with fee [\#449](https://github.com/input-output-hk/jormungandr/issues/449)
- Can't read secret key for creating witness with jcli [\#448](https://github.com/input-output-hk/jormungandr/issues/448)

**Closed issues:**

- jcli: remove 'allow_account_creation' from the config generated with 'jcli genesis init \> genesis.yaml' [\#471](https://github.com/input-output-hk/jormungandr/issues/471)
- Invalid Node secret file: bft.signing_key: Invalid prefix: expected ed25519e_sk but was ed25519_sk at line 6 column 16 [\#460](https://github.com/input-output-hk/jormungandr/issues/460)
- remove the shell ansi colours from scripts/stakepool-single-node-test [\#441](https://github.com/input-output-hk/jormungandr/issues/441)

**Merged pull requests:**

- jcli: 'remove allow_account_creation' from 'jcli genesis init' [\#477](https://github.com/input-output-hk/jormungandr/pull/477)
- Mention add-certificate in stake delegation [\#476](https://github.com/input-output-hk/jormungandr/pull/476)
- Last minute updates [\#474](https://github.com/input-output-hk/jormungandr/pull/474)
- Update to API changes in network-grpc [\#468](https://github.com/input-output-hk/jormungandr/pull/468)
- enable fixing the builds under nix, by making the jormungandr path configurable [\#464](https://github.com/input-output-hk/jormungandr/pull/464)
- Bft secretkey cleanup [\#462](https://github.com/input-output-hk/jormungandr/pull/462)
- Add a full transaction creation and sending example to the docs [\#459](https://github.com/input-output-hk/jormungandr/pull/459)
- Fix error when the current epoch is nearly finished and no block have been created [\#458](https://github.com/input-output-hk/jormungandr/pull/458)
- update cardano-deps and fix issue with fee check [\#455](https://github.com/input-output-hk/jormungandr/pull/455)
- Trim strings read with JCLI read_line [\#454](https://github.com/input-output-hk/jormungandr/pull/454)
- Adding a utility that'll convert a between different addresses [\#453](https://github.com/input-output-hk/jormungandr/pull/453)
- Added scripts for bft node and send transaction [\#445](https://github.com/input-output-hk/jormungandr/pull/445)
- Update network-grpc, ported to tower-hyper [\#444](https://github.com/input-output-hk/jormungandr/pull/444)
- new test case for genesis utxo stake pool [\#443](https://github.com/input-output-hk/jormungandr/pull/443)
- improve jcli account-id parsing [\#442](https://github.com/input-output-hk/jormungandr/pull/442)
- remove stake key and related certificate, fix network compilation [\#440](https://github.com/input-output-hk/jormungandr/pull/440)

\* _This Change Log was automatically generated by [github_changelog_generator](https://github.com/skywinder/Github-Changelog-Generator)_
