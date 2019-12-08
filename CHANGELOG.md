# Change Log

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

- 0.8 rc3 - rewards still not working due to leader\_logs.total or subsequent panic [\#1242](https://github.com/input-output-hk/jormungandr/issues/1242)
- update chain-deps and fix reward distribution panic [\#1246](https://github.com/input-output-hk/jormungandr/pull/1246)
- properly report error and failures of the terminating service [\#1243](https://github.com/input-output-hk/jormungandr/pull/1243)

**Closed issues:**

- bootstrap.sh - error: The following required arguments were not provided:     --serial \<SERIAL\> [\#1244](https://github.com/input-output-hk/jormungandr/issues/1244)

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
- Optimize locking with Storage::send\_from\_to [\#1209](https://github.com/input-output-hk/jormungandr/pull/1209)
- rename blockchain\_stuck\_notifier [\#1208](https://github.com/input-output-hk/jormungandr/pull/1208)
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
- Rise type\_length\_limit [\#1162](https://github.com/input-output-hk/jormungandr/pull/1162)
- catch the error the drains [\#1154](https://github.com/input-output-hk/jormungandr/pull/1154)
- Make sure if the node wake too early for the leader event to wait a bit [\#1151](https://github.com/input-output-hk/jormungandr/pull/1151)

**Closed issues:**

- Local 0.7.0 Jormungandr node not receiving blocks from Incentivized TestNet [\#1147](https://github.com/input-output-hk/jormungandr/issues/1147)
- bootstrap script fails [\#1142](https://github.com/input-output-hk/jormungandr/issues/1142)
- Which faucet to use for the latest release v0.7.0? [\#1135](https://github.com/input-output-hk/jormungandr/issues/1135)
- Error fetching the genesis block from the network [\#1132](https://github.com/input-output-hk/jormungandr/issues/1132)
- panic with error: Some\(NonMonotonicDate [\#1130](https://github.com/input-output-hk/jormungandr/issues/1130)
- Documentation to register a stake pool is not up to date [\#1110](https://github.com/input-output-hk/jormungandr/issues/1110)
- panicked at 'upper\_bound should be \>= than lower\_bound' [\#1093](https://github.com/input-output-hk/jormungandr/issues/1093)

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
- logging settings: .async\(\) -\> .into\_async\(\) [\#1119](https://github.com/input-output-hk/jormungandr/pull/1119)
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
- error while streaming response: Error { code: Internal, cause: CannotIterate }, sub\_task: server, task: network [\#1056](https://github.com/input-output-hk/jormungandr/issues/1056)
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
- Error in the overall configuration of the node  |-\> Error while parsing the node configuration file: p2p.trusted\_peers\[0\].id: Odd number of digits at line 13 column 11  |-\> p2p.trusted\_peers\[0\].id: Odd number of digits at line 13 column 11 [\#1094](https://github.com/input-output-hk/jormungandr/issues/1094)
- Mined transactions are not propagated to other nodes - RC5, local cluster with 2 nodes [\#1090](https://github.com/input-output-hk/jormungandr/issues/1090)
- Transactions not propagating [\#1042](https://github.com/input-output-hk/jormungandr/issues/1042)
- Sync issues not fully resolved [\#1031](https://github.com/input-output-hk/jormungandr/issues/1031)

**Merged pull requests:**

- breaking change: add discriminant account signature and reward account [\#1116](https://github.com/input-output-hk/jormungandr/pull/1116)
- Port Jormungandr to 2018 edition [\#1115](https://github.com/input-output-hk/jormungandr/pull/1115)
- fixed create-account-and-delegate.shtmpl to work with 0.7.0-rc7 [\#1111](https://github.com/input-output-hk/jormungandr/pull/1111)
- Add old utxo and old address to explorer [\#1109](https://github.com/input-output-hk/jormungandr/pull/1109)
- Update Quick-Start in public mode section [\#1108](https://github.com/input-output-hk/jormungandr/pull/1108)
- Simplify Storage::stream\_from\_to [\#1102](https://github.com/input-output-hk/jormungandr/pull/1102)
- registering\_stake\_pool.md: do not longer sign the cert [\#1101](https://github.com/input-output-hk/jormungandr/pull/1101)

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
- doc: Remove remaining traces of private\_id [\#1079](https://github.com/input-output-hk/jormungandr/pull/1079)
- \[Tests \] Stats rest method for node [\#1077](https://github.com/input-output-hk/jormungandr/pull/1077)
- Implement a custom policy object [\#1076](https://github.com/input-output-hk/jormungandr/pull/1076)
- remove unused imports [\#1075](https://github.com/input-output-hk/jormungandr/pull/1075)
- \[Tests\] Use KeyPair from jormungandr-libs [\#1074](https://github.com/input-output-hk/jormungandr/pull/1074)
- Restore REST TX info functionality [\#1070](https://github.com/input-output-hk/jormungandr/pull/1070)
- jcli: certificate print - also signedcert [\#1068](https://github.com/input-output-hk/jormungandr/pull/1068)
- Update introduction.md [\#1062](https://github.com/input-output-hk/jormungandr/pull/1062)
- Docs: allow\_private\_addresses - add to sample cfg [\#1059](https://github.com/input-output-hk/jormungandr/pull/1059)
- \[Tests\] Testnet test case stability fix [\#1051](https://github.com/input-output-hk/jormungandr/pull/1051)
- \[Tests\] Jormungandr-scenario-tests added grpc calls to node [\#1047](https://github.com/input-output-hk/jormungandr/pull/1047)
- \[Tests\] Genesis initial  test cases [\#1023](https://github.com/input-output-hk/jormungandr/pull/1023)

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
- \[Tests\] add error\_chain to grpc mock [\#1045](https://github.com/input-output-hk/jormungandr/pull/1045)
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

- Cannot encode genesis.yaml with legacy\_funds entry [\#968](https://github.com/input-output-hk/jormungandr/issues/968)
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

- doc: private\_id is optional, used for trusted peers [\#959](https://github.com/input-output-hk/jormungandr/pull/959)

## [v0.6.2](https://github.com/input-output-hk/jormungandr/tree/v0.6.2) (2019-10-18)
[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.6.1...v0.6.2)

**Implemented enhancements:**

- Add --rest-listen-address Jormungandr CLI arg [\#925](https://github.com/input-output-hk/jormungandr/pull/925)

**Fixed bugs:**

- The existential terror of non existence - a soliloquy.  [\#941](https://github.com/input-output-hk/jormungandr/issues/941)

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
- Fix REST server panic when get\_block\_id gets nonexistent block ID [\#912](https://github.com/input-output-hk/jormungandr/pull/912)
- JCLI: transaction info - remove args positionality [\#910](https://github.com/input-output-hk/jormungandr/pull/910)
- Put a hard limit on incoming p2p connections [\#909](https://github.com/input-output-hk/jormungandr/pull/909)

**Fixed bugs:**

- task: leadership thread 'leadership2' panicked  \(node continue to run\) [\#930](https://github.com/input-output-hk/jormungandr/issues/930)
- Suspected file descriptor leak \(ERRO Error while accepting connection on 0.0.0.0:3000: Os\) [\#923](https://github.com/input-output-hk/jormungandr/issues/923)
- Querying the node for an inexisting block data, panics! [\#859](https://github.com/input-output-hk/jormungandr/issues/859)
- initial bootstrap completedthread 'network, ' panicked at 'not yet implementedpeer\_addr' [\#849](https://github.com/input-output-hk/jormungandr/issues/849)
- leadership module does not purge logs. [\#842](https://github.com/input-output-hk/jormungandr/issues/842)
- Fix and improvements in the fragment logs handling [\#931](https://github.com/input-output-hk/jormungandr/pull/931)
- Fix REST server panic when get\\_block\\_id gets nonexistent block ID [\#912](https://github.com/input-output-hk/jormungandr/pull/912)
- Fix end and start cursors in block connection [\#904](https://github.com/input-output-hk/jormungandr/pull/904)

**Closed issues:**

- /api/v0/account/{account\_id}  truncating address resulting in 404 not found [\#908](https://github.com/input-output-hk/jormungandr/issues/908)

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
- Jormungandr: enrich rest get\_settings [\#887](https://github.com/input-output-hk/jormungandr/pull/887)
- Add certificate query for transaction [\#878](https://github.com/input-output-hk/jormungandr/pull/878)

**Fixed bugs:**

- Errors reported in logs on 0.5.4 [\#867](https://github.com/input-output-hk/jormungandr/issues/867)
- Return HTTP 400 from next\_id when block is not in tip chain [\#894](https://github.com/input-output-hk/jormungandr/pull/894)

**Closed issues:**

- 500 Internal Server error querying next-id [\#888](https://github.com/input-output-hk/jormungandr/issues/888)
- Connection refused \(os error 111\) [\#886](https://github.com/input-output-hk/jormungandr/issues/886)
- My local node/stake pool did not create any block [\#840](https://github.com/input-output-hk/jormungandr/issues/840)

**Merged pull requests:**

- Handle listening error in run\_listen\_socket [\#901](https://github.com/input-output-hk/jormungandr/pull/901)
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

- JCLI: management\_threshold validity [\#838](https://github.com/input-output-hk/jormungandr/pull/838)

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

- `lastBlockTime` parameter \(for jcli rest v0 node stats\) does not return any value  [\#834](https://github.com/input-output-hk/jormungandr/issues/834)
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
- Make bump\_peer\_for\_block\_fetch miss non-fatal [\#811](https://github.com/input-output-hk/jormungandr/pull/811)
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
- Server Error when directing rest calls to public\_address instead of rest address [\#775](https://github.com/input-output-hk/jormungandr/issues/775)

**Merged pull requests:**

- Improve processing of inbound subscription streams [\#789](https://github.com/input-output-hk/jormungandr/pull/789)
- network: Replace forward combinator with send\_all [\#787](https://github.com/input-output-hk/jormungandr/pull/787)
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
- \[Documentation\] Remove public\_id from docs [\#758](https://github.com/input-output-hk/jormungandr/pull/758)
- Remove Id from the poldercast gossiping [\#757](https://github.com/input-output-hk/jormungandr/pull/757)
- Offer content service on the server Node [\#755](https://github.com/input-output-hk/jormungandr/pull/755)
- explicit compilation of the node and jcli [\#751](https://github.com/input-output-hk/jormungandr/pull/751)
- make sure we don't build the integration tests dependencies unless needed [\#750](https://github.com/input-output-hk/jormungandr/pull/750)
- go through the whole list of trusted peers on the network [\#749](https://github.com/input-output-hk/jormungandr/pull/749)
- Process fragment subscription on the server side [\#747](https://github.com/input-output-hk/jormungandr/pull/747)
- Add stub to process fragment subscription [\#743](https://github.com/input-output-hk/jormungandr/pull/743)
- Update chain-deps, use ContentService to subscribe the client to fragments [\#740](https://github.com/input-output-hk/jormungandr/pull/740)
- Fragment process clean up [\#737](https://github.com/input-output-hk/jormungandr/pull/737)
- make the node use multiaddr for the listen\_addr [\#736](https://github.com/input-output-hk/jormungandr/pull/736)
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
- doc: renaming genesis to genesis\_praos [\#721](https://github.com/input-output-hk/jormungandr/pull/721)
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

- \[Jormungandr\] - \[mempool\] : Node "stops" producing blocks if garbage\_collection\_interval \< fragment\_ttl [\#705](https://github.com/input-output-hk/jormungandr/issues/705)
- Database error after abrupt node restart [\#676](https://github.com/input-output-hk/jormungandr/issues/676)
- make sure we don't  block the poll in the fragment pool too [\#706](https://github.com/input-output-hk/jormungandr/pull/706)
- Mempool and Leadership logs GC setting and fixes [\#703](https://github.com/input-output-hk/jormungandr/pull/703)
- Fix tests aborting on invalid logs [\#689](https://github.com/input-output-hk/jormungandr/pull/689)
- Added block processing for new blockchain [\#684](https://github.com/input-output-hk/jormungandr/pull/684)

**Closed issues:**

- serve the leader logs through the Rest API [\#698](https://github.com/input-output-hk/jormungandr/issues/698)

**Merged pull requests:**

- Testing scenario managing test flow [\#716](https://github.com/input-output-hk/jormungandr/pull/716)
- Testing scenario managing test flow [\#715](https://github.com/input-output-hk/jormungandr/pull/715)
- Rest async [\#714](https://github.com/input-output-hk/jormungandr/pull/714)
- \[Tests\] Fixed test\_genesis\_stake\_pool\_with\_utxo\_faucet\_starts\_successfully [\#713](https://github.com/input-output-hk/jormungandr/pull/713)
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
- Extend logs with app version, epoch and slot\_id [\#679](https://github.com/input-output-hk/jormungandr/pull/679)
- Graceful handling of block0 in the future [\#661](https://github.com/input-output-hk/jormungandr/pull/661)
- Add stake pool getter to REST API [\#660](https://github.com/input-output-hk/jormungandr/pull/660)
- network: Perform protocol handshake [\#657](https://github.com/input-output-hk/jormungandr/pull/657)
- Add leadership management REST API [\#654](https://github.com/input-output-hk/jormungandr/pull/654)

**Fixed bugs:**

- Upgrade custom\_error to 1.7.1 [\#678](https://github.com/input-output-hk/jormungandr/pull/678)
- it seems that debug\_assertions feature was not doing what I expected [\#677](https://github.com/input-output-hk/jormungandr/pull/677)
- Graceful handling of block0 in the future [\#661](https://github.com/input-output-hk/jormungandr/pull/661)
- Poll gRPC client ready before sending any requests [\#656](https://github.com/input-output-hk/jormungandr/pull/656)
- Don't let one client connection terminate task [\#650](https://github.com/input-output-hk/jormungandr/pull/650)

**Closed issues:**

- Jcli: address info - wrong subcommand description [\#670](https://github.com/input-output-hk/jormungandr/issues/670)
- jormungandr install error [\#665](https://github.com/input-output-hk/jormungandr/issues/665)
- Jcli: cargo install failure due to custom\_error/1.7.1 crate [\#664](https://github.com/input-output-hk/jormungandr/issues/664)
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
- Boxing problem with custom\_error [\#662](https://github.com/input-output-hk/jormungandr/pull/662)
- Network fixes [\#655](https://github.com/input-output-hk/jormungandr/pull/655)
- Protocol doc update [\#653](https://github.com/input-output-hk/jormungandr/pull/653)
- Fixed test\_correct\_utxo\_transaction\_replaces\_old\_utxo\_by\_node test [\#651](https://github.com/input-output-hk/jormungandr/pull/651)
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
- Simplify slot\_start\_time storage to seconds [\#636](https://github.com/input-output-hk/jormungandr/pull/636)
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
- Clean up unnecessary lifetimes in configuration\_builder test tools [\#630](https://github.com/input-output-hk/jormungandr/pull/630)
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
- bootstrap error in genesis\_praos, genesis file corrupted [\#562](https://github.com/input-output-hk/jormungandr/issues/562)
- Documentation: Improve the documentation related to Staking&Delegation  [\#530](https://github.com/input-output-hk/jormungandr/issues/530)
- documentation: Add a consolidated/consistent/easier way for starting the node [\#515](https://github.com/input-output-hk/jormungandr/issues/515)
- documentation: improve the documentation for 'jcli rest v0 account get' [\#484](https://github.com/input-output-hk/jormungandr/issues/484)

**Merged pull requests:**

- Finalize Divide and Reuse [\#600](https://github.com/input-output-hk/jormungandr/pull/600)
- More changes in the jormungandr-lib API [\#599](https://github.com/input-output-hk/jormungandr/pull/599)
- take into account the certificate when computing the fees [\#598](https://github.com/input-output-hk/jormungandr/pull/598)
- Use certificate from jormungandr lib [\#597](https://github.com/input-output-hk/jormungandr/pull/597)
- Improve testing of the Block0Configuration [\#596](https://github.com/input-output-hk/jormungandr/pull/596)
- updated delegation script  [\#595](https://github.com/input-output-hk/jormungandr/pull/595)
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

- jcli 0.2.1 \[jcli key generate --type\]  [\#501](https://github.com/input-output-hk/jormungandr/issues/501)
- REST account API: The delegation field format should be improved [\#491](https://github.com/input-output-hk/jormungandr/issues/491)
- gelf logging support for slog [\#447](https://github.com/input-output-hk/jormungandr/issues/447)

**Merged pull requests:**

- mark gelf as optional feature [\#557](https://github.com/input-output-hk/jormungandr/pull/557)
- Fix incorrect PATH setting [\#555](https://github.com/input-output-hk/jormungandr/pull/555)
- Update introduction.md [\#552](https://github.com/input-output-hk/jormungandr/pull/552)
- Update delegating\_stake.md [\#550](https://github.com/input-output-hk/jormungandr/pull/550)
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
- Fix README typo: public\_access-\>public\_address [\#482](https://github.com/input-output-hk/jormungandr/pull/482)
- add option to disable colours, fix find for deleting tmp files [\#480](https://github.com/input-output-hk/jormungandr/pull/480)
- Stake key certificate does not exist anymore [\#461](https://github.com/input-output-hk/jormungandr/pull/461)

## [v0.2.0](https://github.com/input-output-hk/jormungandr/tree/v0.2.0) (2019-06-13)
[Full Changelog](https://github.com/input-output-hk/jormungandr/compare/v0.1.0...v0.2.0)

**Fixed bugs:**

- Error when verifying transaction with fee [\#449](https://github.com/input-output-hk/jormungandr/issues/449)
- Can't read secret key for creating witness with jcli [\#448](https://github.com/input-output-hk/jormungandr/issues/448)

**Closed issues:**

- jcli: remove 'allow\_account\_creation' from the config generated with 'jcli genesis init \> genesis.yaml'  [\#471](https://github.com/input-output-hk/jormungandr/issues/471)
- Invalid Node secret file: bft.signing\_key: Invalid prefix: expected ed25519e\_sk but was ed25519\_sk at line 6 column 16 [\#460](https://github.com/input-output-hk/jormungandr/issues/460)
- remove the shell ansi colours from scripts/stakepool-single-node-test [\#441](https://github.com/input-output-hk/jormungandr/issues/441)

**Merged pull requests:**

- jcli: 'remove allow\_account\_creation' from 'jcli genesis init' [\#477](https://github.com/input-output-hk/jormungandr/pull/477)
- Mention add-certificate in stake delegation [\#476](https://github.com/input-output-hk/jormungandr/pull/476)
- Last minute updates [\#474](https://github.com/input-output-hk/jormungandr/pull/474)
- Update to API changes in network-grpc [\#468](https://github.com/input-output-hk/jormungandr/pull/468)
- enable fixing the builds under nix, by making the jormungandr path configurable [\#464](https://github.com/input-output-hk/jormungandr/pull/464)
- Bft secretkey cleanup [\#462](https://github.com/input-output-hk/jormungandr/pull/462)
- Add a full transaction creation and sending example to the docs [\#459](https://github.com/input-output-hk/jormungandr/pull/459)
- Fix error when the current epoch is nearly finished and no block have been created [\#458](https://github.com/input-output-hk/jormungandr/pull/458)
- update cardano-deps and fix issue with fee check [\#455](https://github.com/input-output-hk/jormungandr/pull/455)
- Trim strings read with JCLI read\_line [\#454](https://github.com/input-output-hk/jormungandr/pull/454)
- Adding a utility that'll convert a between different addresses [\#453](https://github.com/input-output-hk/jormungandr/pull/453)
- Added scripts for bft node and send transaction [\#445](https://github.com/input-output-hk/jormungandr/pull/445)
- Update network-grpc, ported to tower-hyper [\#444](https://github.com/input-output-hk/jormungandr/pull/444)
- new test case for genesis utxo stake pool [\#443](https://github.com/input-output-hk/jormungandr/pull/443)
- improve jcli account-id parsing  [\#442](https://github.com/input-output-hk/jormungandr/pull/442)
- remove stake key and related certificate, fix network compilation [\#440](https://github.com/input-output-hk/jormungandr/pull/440)



\* *This Change Log was automatically generated by [github_changelog_generator](https://github.com/skywinder/Github-Changelog-Generator)*
