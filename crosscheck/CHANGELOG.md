# Changelog

## [2.9.0](https://github.com/nicholls-inc/crosscheck/compare/crosscheck-v2.8.0...crosscheck-v2.9.0) (2026-10-07)


### Features

* **crosscheck:** check slash-references in pre-commit and CI and generate the skill catalogue ([#108](https://github.com/nicholls-inc/crosscheck/issues/108)) ([deed99a](https://github.com/nicholls-inc/crosscheck/commit/deed99a43a3e95024b8afdb35a74391194b10eae))
* **crosscheck:** check that dafny_evidence's requirement names a tracked file, and refuse a duplicate theorem ([#97](https://github.com/nicholls-inc/crosscheck/issues/97)) ([e5089f7](https://github.com/nicholls-inc/crosscheck/commit/e5089f7c9cddf7ebebb65c1d051041bba96a0103))
* **crosscheck:** emit an evidence record from /generate-verified with dafny_evidence ([#91](https://github.com/nicholls-inc/crosscheck/issues/91)) ([b5893ea](https://github.com/nicholls-inc/crosscheck/commit/b5893ea83d3aa695f0f877b4823df4ad48878c38))
* **crosscheck:** emit an evidence record from the Dafny pipeline with dafny_evidence ([#82](https://github.com/nicholls-inc/crosscheck/issues/82)) ([0f3ab3d](https://github.com/nicholls-inc/crosscheck/commit/0f3ab3d1840fb1bc3284d8efc8c434882f45b294))
* **crosscheck:** let a maintainer's instruction tick a verification box and tick mechanical items with evidence ([#127](https://github.com/nicholls-inc/crosscheck/issues/127)) ([f72d959](https://github.com/nicholls-inc/crosscheck/commit/f72d959433c83ce9a007b5eb263e5907fcedbca5))
* **crosscheck:** rerun dafny_evidence records by image ID, and narrow output and include paths ([#95](https://github.com/nicholls-inc/crosscheck/issues/95)) ([5dfb277](https://github.com/nicholls-inc/crosscheck/commit/5dfb277402780fd555716d2dd689833d935fec0b))
* **crosscheck:** run dafny_evidence and its rerun command with sandbox flags ([#93](https://github.com/nicholls-inc/crosscheck/issues/93)) ([17d40ec](https://github.com/nicholls-inc/crosscheck/commit/17d40ec21aed6e113193b109853ef9ae35a81996))
* **crosscheck:** run dafny_evidence's Dafny on a copy of the blobs at commit ([#118](https://github.com/nicholls-inc/crosscheck/issues/118)) ([d261934](https://github.com/nicholls-inc/crosscheck/commit/d261934dacfb1cb59025018ed8d18a234476abab))


### Bug Fixes

* **crosscheck:** fail the conformance run on a claims.json of the wrong shape ([#114](https://github.com/nicholls-inc/crosscheck/issues/114)) ([bdae608](https://github.com/nicholls-inc/crosscheck/commit/bdae6086617ef7fc92bb74d804f7f2d289322969))
* **crosscheck:** fail the conformance run on a dangling claims.json symlink ([#113](https://github.com/nicholls-inc/crosscheck/issues/113)) ([904d948](https://github.com/nicholls-inc/crosscheck/commit/904d948167a3f2927570def44e7e2973deb29348))
* **crosscheck:** fail the conformance run on a plugin root that does not resolve ([#115](https://github.com/nicholls-inc/crosscheck/issues/115)) ([4a9f341](https://github.com/nicholls-inc/crosscheck/commit/4a9f341c2984bc25f8b691048e88d992bb491607))
* **crosscheck:** fail the conformance run on a root that is not a Crosscheck plugin tree ([#119](https://github.com/nicholls-inc/crosscheck/issues/119)) ([60e1a43](https://github.com/nicholls-inc/crosscheck/commit/60e1a43c9af9dc9751d065ec7c9a927dd32bb80a))
* **crosscheck:** fail the conformance run on an unreadable or invalid claims.json ([#111](https://github.com/nicholls-inc/crosscheck/issues/111)) ([9f99147](https://github.com/nicholls-inc/crosscheck/commit/9f991479f249b394d55dca8bce459f9811b2302b))
* **crosscheck:** give the invariant-coverage gate one heading grammar ([#112](https://github.com/nicholls-inc/crosscheck/issues/112)) ([8138901](https://github.com/nicholls-inc/crosscheck/commit/81389013f13d728dd5cf774f8ff2a99501015567))
* **crosscheck:** reject a claims.json with missing fields, unknown check types or duplicate keys ([#117](https://github.com/nicholls-inc/crosscheck/issues/117)) ([24bc384](https://github.com/nicholls-inc/crosscheck/commit/24bc384764fbda056260c0410669253a6df477c3))
* **crosscheck:** reject a ledger claim status outside the allowlist ([#110](https://github.com/nicholls-inc/crosscheck/issues/110)) ([7cafa0d](https://github.com/nicholls-inc/crosscheck/commit/7cafa0d26c3e5bc37f0ed837ac2e44af36346527))
* **crosscheck:** reject two claims with the same id in claims.json ([#120](https://github.com/nicholls-inc/crosscheck/issues/120)) ([bdb5ca6](https://github.com/nicholls-inc/crosscheck/commit/bdb5ca6c23ddc1ae64f0c4fde555cb56ab6ab9a3))
* **crosscheck:** require a skill and an agent under a plugin root, and read the manifest name key exactly ([#121](https://github.com/nicholls-inc/crosscheck/issues/121)) ([4f0c1e0](https://github.com/nicholls-inc/crosscheck/commit/4f0c1e05581b8ec633bf23d47542d2c25a271160))
* **crosscheck:** say "not yet reached" for Layer 6 and unreached classes in skills ([#77](https://github.com/nicholls-inc/crosscheck/issues/77)) ([2cfb0f5](https://github.com/nicholls-inc/crosscheck/commit/2cfb0f5119bb5b7219f0ff7893f807c334ddcc46))
* **crosscheck:** stop merged governance notes from unlocking the protected-surface hook ([#55](https://github.com/nicholls-inc/crosscheck/issues/55)) ([2884b50](https://github.com/nicholls-inc/crosscheck/commit/2884b502178ac927771115f092316361119a0d30))
* **crosscheck:** stop presenting the intent-check attestation as a required artefact ([#72](https://github.com/nicholls-inc/crosscheck/issues/72)) ([05180a7](https://github.com/nicholls-inc/crosscheck/commit/05180a7cfab8726514e7f127bdebf9e263683de9))
* **crosscheck:** write ledger parse faults in the checker's own words and reject invisible text ([#122](https://github.com/nicholls-inc/crosscheck/issues/122)) ([cfbbf38](https://github.com/nicholls-inc/crosscheck/commit/cfbbf38984ea5a155a779690aef6667319f65702))

## [2.8.0](https://github.com/nicholls-inc/crosscheck/compare/crosscheck-v2.7.0...crosscheck-v2.8.0) (2026-09-29)


### Features

* **crosscheck:** import the plugin and its framework with history ([e76ac19](https://github.com/nicholls-inc/crosscheck/commit/e76ac19329321c14d579ea55ad2956b80768d1f7))

## [2.7.0](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.6.0...crosscheck-v2.7.0) (2026-08-25)


### Features

* **crosscheck:** add self-explanatory gate messages and playbook scaffolding to skills and agents ([#246](https://github.com/nicholls-inc/claude-code-marketplace/issues/246)) ([8945d63](https://github.com/nicholls-inc/claude-code-marketplace/commit/8945d63132644e37e3b8f93e78518f243b6ca19e))

## [2.6.0](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.5.1...crosscheck-v2.6.0) (2026-06-01)


### Features

* **crosscheck:** add conformance/inventory oracle + fix assurance-probe frontmatter ([#214](https://github.com/nicholls-inc/claude-code-marketplace/issues/214)) ([b67d0cb](https://github.com/nicholls-inc/claude-code-marketplace/commit/b67d0cb87be7e6b3f62cfdd4d3dbdbd842eb6f66))
* **crosscheck:** add orchestration-graph integrity self-check (trunk coverage, [#221](https://github.com/nicholls-inc/claude-code-marketplace/issues/221)) ([#230](https://github.com/nicholls-inc/claude-code-marketplace/issues/230)) ([d42600c](https://github.com/nicholls-inc/claude-code-marketplace/commit/d42600c99ba7616fd8b76318e4a98bd8b51db486))
* **crosscheck:** document journal-context and track ledger gaps in conformance oracle ([#216](https://github.com/nicholls-inc/claude-code-marketplace/issues/216)) ([b04f7e0](https://github.com/nicholls-inc/claude-code-marketplace/commit/b04f7e04bb77e4097008e1e943056b13c1a7b8e0))
* **crosscheck:** draft A1-A6 acceptance oracles for ratification (no Phase 4 build) ([#223](https://github.com/nicholls-inc/claude-code-marketplace/issues/223)) ([e917857](https://github.com/nicholls-inc/claude-code-marketplace/commit/e917857e3ba6e84f697f5e82e030c400ef132adf))
* **crosscheck:** ship the Phase 4 gated-implementation agent (lowry) ([#218](https://github.com/nicholls-inc/claude-code-marketplace/issues/218)) ([#233](https://github.com/nicholls-inc/claude-code-marketplace/issues/233)) ([fda5b7e](https://github.com/nicholls-inc/claude-code-marketplace/commit/fda5b7ecdd8ef2e5f37d5674d0e6d2d90bd4f2c4))
* **crosscheck:** ship the Phase 5 Auditor agent (auditor) ([#220](https://github.com/nicholls-inc/claude-code-marketplace/issues/220)) ([#234](https://github.com/nicholls-inc/claude-code-marketplace/issues/234)) ([2c9d255](https://github.com/nicholls-inc/claude-code-marketplace/commit/2c9d255b42222305d96b4f1e6f70f7d4eeb3d5ef))
* **crosscheck:** wire ADD operating modes + bootstrap/greenfield entrypoints ([#219](https://github.com/nicholls-inc/claude-code-marketplace/issues/219)) ([#232](https://github.com/nicholls-inc/claude-code-marketplace/issues/232)) ([c0beb13](https://github.com/nicholls-inc/claude-code-marketplace/commit/c0beb134ce37e8e8bac724f67805854192ac20f4))


### Bug Fixes

* **add:** unify invariant heading convention on h2 `## I<N>:` across the toolchain ([#228](https://github.com/nicholls-inc/claude-code-marketplace/issues/228)) ([c3fd281](https://github.com/nicholls-inc/claude-code-marketplace/commit/c3fd28143034f65e326890a69ea1902f82fe7515))

## [2.5.1](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.5.0...crosscheck-v2.5.1) (2026-05-28)


### Bug Fixes

* **crosscheck:** raise byfuglien and hellebuyck agent turn limits ([#212](https://github.com/nicholls-inc/claude-code-marketplace/issues/212)) ([96aeadd](https://github.com/nicholls-inc/claude-code-marketplace/commit/96aeadd730a04fb72ebb2e1ef41d0f4b1a69594b))

## [2.5.0](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.4.0...crosscheck-v2.5.0) (2026-05-14)


### Miscellaneous Chores

* **crosscheck:** release 2.5.0 and enforce feat/fix on behavioral artifacts ([#202](https://github.com/nicholls-inc/claude-code-marketplace/issues/202)) ([814ae61](https://github.com/nicholls-inc/claude-code-marketplace/commit/814ae6110d14cdcdec24bef79762e1a492afe4eb))

## [2.4.0](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.3.0...crosscheck-v2.4.0) (2026-05-12)


### Features

* **crosscheck:** /audit-spec-coverage and /audit-invariant-consistency skills ([#179](https://github.com/nicholls-inc/claude-code-marketplace/issues/179)) ([d5b912a](https://github.com/nicholls-inc/claude-code-marketplace/commit/d5b912a1f8ba0639fabc3dc4ae447e3f52eba560))
* **crosscheck:** add-orchestrator agent (ADD methodology workflow runner) ([#181](https://github.com/nicholls-inc/claude-code-marketplace/issues/181)) ([e76cf46](https://github.com/nicholls-inc/claude-code-marketplace/commit/e76cf4610bbc3902a3a6aa530977e4977b72dd17))
* **crosscheck:** orchestrator-marker mode for /draft-invariants ([#180](https://github.com/nicholls-inc/claude-code-marketplace/issues/180)) ([320777e](https://github.com/nicholls-inc/claude-code-marketplace/commit/320777e28bb60457c086d52a9ab4e6762d919a42))


### Bug Fixes

* **crosscheck:** post-validation skill + agent fixes (D2 + D4) ([#183](https://github.com/nicholls-inc/claude-code-marketplace/issues/183)) ([17059d5](https://github.com/nicholls-inc/claude-code-marketplace/commit/17059d5ff7cd03bd6cefefbb1427bbada0704cb8))

## [2.3.0](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.2.0...crosscheck-v2.3.0) (2026-05-11)


### Features

* **crosscheck:** /journal-context skill (JOURNAL.md walk-up) ([#177](https://github.com/nicholls-inc/claude-code-marketplace/issues/177)) ([8ec2ff1](https://github.com/nicholls-inc/claude-code-marketplace/commit/8ec2ff1227088785798c3dd2e5a456c136955b8e))
* **crosscheck:** /rationale C0 trust-boundary top-level branch ([#173](https://github.com/nicholls-inc/claude-code-marketplace/issues/173)) ([e467c41](https://github.com/nicholls-inc/claude-code-marketplace/commit/e467c4183169a86ab7ada46706ce6e7e0b8aec26))
* **crosscheck:** /rationale FORMAL routing Layer 1 vs Layer 4 ([#174](https://github.com/nicholls-inc/claude-code-marketplace/issues/174)) ([bb41e32](https://github.com/nicholls-inc/claude-code-marketplace/commit/bb41e3224b23ddd71f283f2584c572c4cf0af91c))
* **crosscheck:** add /draft-invariants skill (spec-aware) ([#148](https://github.com/nicholls-inc/claude-code-marketplace/issues/148)) ([cfd30c1](https://github.com/nicholls-inc/claude-code-marketplace/commit/cfd30c1701f2f9c270e0bfb9738c7b17b3b6d837))
* **crosscheck:** add Layer 4 Dafny spec for extractDifficultyMetrics ([#136](https://github.com/nicholls-inc/claude-code-marketplace/issues/136)) ([7f4f9fb](https://github.com/nicholls-inc/claude-code-marketplace/commit/7f4f9fbcc48320687b6b75fec7227b979abd5445))
* **crosscheck:** implementation of /assurance-probe skill ([#141](https://github.com/nicholls-inc/claude-code-marketplace/issues/141)) ([4d45de7](https://github.com/nicholls-inc/claude-code-marketplace/commit/4d45de7e874720251ac47ef9ff19e19c2640cde8))
* **crosscheck:** Layer 4 Dafny specs for parseDafnyOutput and shouldExclude ([#134](https://github.com/nicholls-inc/claude-code-marketplace/issues/134)) ([b8f9bb5](https://github.com/nicholls-inc/claude-code-marketplace/commit/b8f9bb530cb5fc84e2823e682927a71560680926))
* **crosscheck:** Lean executable-model pipeline + layered-assurance framing ([#147](https://github.com/nicholls-inc/claude-code-marketplace/issues/147)) ([c1bcd49](https://github.com/nicholls-inc/claude-code-marketplace/commit/c1bcd49cd0f480e92c6b5aff97e41510656477dc))
* **crosscheck:** threshold framing A4 ([#143](https://github.com/nicholls-inc/claude-code-marketplace/issues/143)) ([9f29cc4](https://github.com/nicholls-inc/claude-code-marketplace/commit/9f29cc4116f633129fe481575963def767f8870e))
* **layer4:** add Dafny L4 spec skeleton for extractDifficultyMetrics ([#135](https://github.com/nicholls-inc/claude-code-marketplace/issues/135)) ([96faa84](https://github.com/nicholls-inc/claude-code-marketplace/commit/96faa84c65fb3c8c3b6527bbd934aad708a0ec43))

## [2.2.0](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.1.0...crosscheck-v2.2.0) (2026-04-24)


### Features

* **crosscheck:** add /acceptance-oracle-draft skill ([#123](https://github.com/nicholls-inc/claude-code-marketplace/issues/123)) ([47f4668](https://github.com/nicholls-inc/claude-code-marketplace/commit/47f46688967d2783b04b36a856d1238a36a225ed))
* **crosscheck:** add /assurance-init skill ([#116](https://github.com/nicholls-inc/claude-code-marketplace/issues/116)) ([1dbff96](https://github.com/nicholls-inc/claude-code-marketplace/commit/1dbff96b257baa24ccf3f446a4253394c3fa75b3))
* **crosscheck:** add /assurance-layer-audit skill ([#115](https://github.com/nicholls-inc/claude-code-marketplace/issues/115)) ([db2bd23](https://github.com/nicholls-inc/claude-code-marketplace/commit/db2bd23e973873d3a60fedc48445b27905c508e7))
* **crosscheck:** add /assurance-roadmap-check skill ([#120](https://github.com/nicholls-inc/claude-code-marketplace/issues/120)) ([8bc7fdf](https://github.com/nicholls-inc/claude-code-marketplace/commit/8bc7fdf374a8d9948ccc59181dfd61d66a29fc35))
* **crosscheck:** add /assurance-status skill ([#114](https://github.com/nicholls-inc/claude-code-marketplace/issues/114)) ([5797502](https://github.com/nicholls-inc/claude-code-marketplace/commit/579750293874ae5bb5bea47e84f6521558389d78))
* **crosscheck:** add /intent-check skill ([#122](https://github.com/nicholls-inc/claude-code-marketplace/issues/122)) ([6c64735](https://github.com/nicholls-inc/claude-code-marketplace/commit/6c647359c6a04f110e4291ace370c12b2cbb3c52))
* **crosscheck:** add /invariant-coverage-scaffold skill ([#121](https://github.com/nicholls-inc/claude-code-marketplace/issues/121)) ([d2c52b0](https://github.com/nicholls-inc/claude-code-marketplace/commit/d2c52b0ff8d5f010c5ab0ddd2afb8d7447339ec5))
* **crosscheck:** add /protected-surface-amend skill ([#118](https://github.com/nicholls-inc/claude-code-marketplace/issues/118)) ([5a8ce83](https://github.com/nicholls-inc/claude-code-marketplace/commit/5a8ce83c0818efda6db5e6aa2f004cd421269bc9))
* **crosscheck:** add /spec-adversary skill ([#119](https://github.com/nicholls-inc/claude-code-marketplace/issues/119)) ([7aa57a5](https://github.com/nicholls-inc/claude-code-marketplace/commit/7aa57a5555ead21748d8c7e03d34e97c0fa4b1db))
* **crosscheck:** add hellebuyck orchestrator agent ([#117](https://github.com/nicholls-inc/claude-code-marketplace/issues/117)) ([c06b0f6](https://github.com/nicholls-inc/claude-code-marketplace/commit/c06b0f603d8fdcdcf115e915f5f0bf3437da8d2e))

## [2.1.0](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.0.1...crosscheck-v2.1.0) (2026-03-13)


### Features

* **crosscheck:** add frontmatter to agent and skills ([#46](https://github.com/nicholls-inc/claude-code-marketplace/issues/46)) ([a143c14](https://github.com/nicholls-inc/claude-code-marketplace/commit/a143c1497a030cb7f7884b3d4bec200613d8b04b))

## [2.0.1](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v2.0.0...crosscheck-v2.0.1) (2026-03-12)


### Bug Fixes

* **crosscheck:** fix agent description ([bbd5679](https://github.com/nicholls-inc/claude-code-marketplace/commit/bbd567995ac51faab405fdf230c8e851b199d03d))

## [2.0.0](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v1.0.2...crosscheck-v2.0.0) (2026-03-12)


### ⚠ BREAKING CHANGES

* **crosscheck:** semiformal/ directory deleted; all skills moved to crosscheck/skills/.

### Features

* **crosscheck:** add spec registry, regression detection, and structured adequacy skills ([#36](https://github.com/nicholls-inc/claude-code-marketplace/issues/36)) ([19a88c8](https://github.com/nicholls-inc/claude-code-marketplace/commit/19a88c8341b2c1d0a6425d01fee06f6027c30d96))

## [1.0.2](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v1.0.1...crosscheck-v1.0.2) (2026-03-10)


### Bug Fixes

* **crosscheck:** update build comments after esbuild migration ([eeeb5a2](https://github.com/nicholls-inc/claude-code-marketplace/commit/eeeb5a2a0a8e95ab517aee2b6e8a0d62c6d97e6d))

## [1.0.1](https://github.com/nicholls-inc/claude-code-marketplace/compare/crosscheck-v1.0.0...crosscheck-v1.0.1) (2026-03-09)


### Bug Fixes

* **crosscheck:** correct release-please extra-files paths and add package.json ([#29](https://github.com/nicholls-inc/claude-code-marketplace/issues/29)) ([fb86a5b](https://github.com/nicholls-inc/claude-code-marketplace/commit/fb86a5bdfb23036459e501518a4dc212cffe6452))
