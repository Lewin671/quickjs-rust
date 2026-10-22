# T064/T056 successor plan inputs (2026-10-22 UTC)

- Status: historical-campaign; input collection and draft review record, not a frozen plan.
- Implementation: none; no production change, timing screen, plan freeze or old-plan closure.
- Verified at: code revision `4fa3f739902bf4a78035e36522f4c2419f7bb45e` plus this documentation delta; full local gate passed; exact M profiles and draft validation below.
- Evidence: [review contract](../proposals/T064-successor-review.md), [closure draft](../proposals/T064-layout-edges-closure.md), [schema-3 draft](../proposals/T064-kernel-layout-authority.json); exact identities below.
- Unresolved: lead approval, K0 integration/qualification and the proposed resource-ceiling choice remain open; no successor is frozen.
- Next action: lead reviews the draft/override/budgets, then freezes it and enacts the old-plan closure together before K1; do not implement or screen from this proposal.

## Input identities and jobs

The formal selection bundle is iMac job
`20261022T041140Z-current-main-9eba7a1d-formal`, exit 0, workers-4, 30 blocks.
M is `9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30`, executable SHA-256
`f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb`.
The supplied queue SHA-256 is
`f6666a68cb8e1841418d6f5012e6f7810b0c25ebf982b45ef6d5564855e0ae13`;
summary SHA-256 is
`205e882999604170609e6e46e98e6c83091604bb9f9b6b2da0b322f817a41bc5`.
Runtime/build inputs are identical between this checkout's base and M (only
T040 documentation differs). The bundle's missing historical-base lodash
comparison makes its overall readiness inconclusive; it does not remove
lodash from the successor's 86-case control inventory.

Sampling job `20261022T050553Z-t064-successor-exact-profiles` was submitted
through `imac-bench.sh run --timing --ref 9eba7a1d` after checking status, behind
the existing K0 and allocation/startup jobs; no job was cancelled. It regenerates
the queue from the sealed bundle and checks byte identity before touching the
candidate cache executable. No local or remote release build is requested by
this collector.

The collector reuses the process/sampler/receipt procedure of
`20261017T095048Z-t056-part2-development-sampling-windows`: 3 repeats per
development case, `sample <pid> 10 1 -mayDie -file <raw.sample>`, then wait for
the real workload to finish and validate its checksum. Each suite/control has
3 repeats with a 2-second sample window; internal cases run 3,000,000 iterations.
The sampler may finish after a short process exits. Development windows may
end before long processes finish. These are sampled stacks, not speed estimates.
Import each window independently through `tools.benchmark.profile.import_profile`
with M and the exact executable, workload and actual sampler argv. No instrumented
image and no held-out workload is sampled or used for selection.

Raw outputs and receipt assets are retained in the queue job directory on the
iMac and the workspace copy `outputs/imac-bench/<job>/out/`. They are external
artifacts, not committed repository files; the measurements, conclusions and
hashes recorded here are the durable repository evidence.

## Collection outcome and artifact hashes

Job `20261022T050553Z-t064-successor-exact-profiles`: **done 0**, 143 seconds
reported by the runner after host quieting (queue wait excluded). All 27
samplers and all 27 processes exit 0; every workload output passes the existing
manifest/screen checksum contract. `intruders.log` is empty: no disturbance or
load replacement. The regenerated queue is byte-identical to the supplied
queue; all receipts name exact M and executable f5822aa3, not an instrumented
build or the old T056 base. No optimization screen/formal decision was run.

| Artifact (relative to job directory) | SHA-256 |
| --- | --- |
| `command.sh` | `8bcfc5b2615724d54e976b57ebe0ca494ede046b9b7425f877624a2a72f76ecd` |
| `intruders.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `out/queue.json` | `f6666a68cb8e1841418d6f5012e6f7810b0c25ebf982b45ef6d5564855e0ae13` |
| `out/selection.json` | `666048e84e430a7edfbd7e1d57e378dd25ee95594085c993e4364f1bd8b5db5c` |
| `out/host.json` | `b6c7c3b696ba08e08e0d05a23f81c0c653baf544b61cab2753254a0c56fb2976` |
| `out/profiles.json` | `125d97459239943526a3ab0d12c452e85e8eeb7e8151bdefb839807de32a3326` |
| `out/analysis.json` | `2df3422c3de9f37576172dc58823c3e6870cfecd41f3ac420f0078b06530fe34` |

`analysis.json` is a local derived summary, not a sealed sampler receipt; the
raw receipts and workload assets below are independently content-verified.
The analysis script is local, not retained in Git; its algorithm and the
measured counts/shares are recorded here. Receipt asset verification through
`tools.benchmark.profile.verify_profiles` passed **27/27**, including the 18
control windows not required by the draft's target-only profile list.

## Attribution method and limits

Read the first `sample` main-thread tree only. Compute each node's self count
as inclusive count minus direct-child counts, asserting nonnegative counts and
that the self counts sum to the thread total. Count a sample once in the named
union if any frame matches the following symbol families (Rust mangled names):

```
PropertyStorage|PropertyLayout|ObjectRef\w*own\w*propert|own_data_propert|
get_prop_named|set_prop_named|op_get_prop|op_set_prop|NamedPropertyCache|
named_property_cache|with_prototype|OrderedDataPropertyBuilder|slot_reads|
property_value|8property|lookup_property|get_property|set_property|define_own|
7Shaped|dynamic_storage|object_literal|ObjectLiteralShape|layout
```

The `vm_props` module name alone is deliberately insufficient: its
`fast_number_binary` arithmetic helper is not named-property work. Including
that whole module would falsely charge 3.68% of broad/property_read to names;
the table below excludes it on semantic grounds, consistently for all cases.

For owner attribution charge allocator, mem*, hash and Value clone/drop/vector
growth self samples to the nearest recognized semantic caller. Classify that
caller into named, env/bindings, Vm construction/prepared-call, call dispatch,
virtual-object, parser/compiler, strings/RegExp, native, loop, array, teardown
or other; these groups partition self samples. Loop includes inlined work
that cannot be separated by symbol. Named includes storage/construction,
prototype and exotic bridges as well as lookup; its inclusive envelope includes
required callees and is not a removable fraction. Do not add inclusive shares
to owner shares or infer a forecast speedup.

Pool counts across all three windows; do not choose a window by its share.
Moment outlives every 10-second window; other processes have finished by
sampler return, with normal attachment/startup coverage limitations. Moment's
observed named envelope spans 25.67–26.55%, diff 31.52–31.80%, meriyah
32.15–34.32% across the three process windows. These are ranges, not confidence
intervals. External/sentinel samples are small diagnostics; no missing-path
claim follows from a zero sampled count.

## Fresh shares and target selection

Named-incl = union envelope; Named-owner = partitioned self ownership.
Vm+env+call and Strings/RegExp are separate owner groups; every percentage is
of all sampled main-thread samples, including startup/teardown if observed.
Ranks are lane-local from this queue, not historical priorities.

| Case | Queue rank / Rust:NG | Samples | Named-incl | Named-owner | Vm+env+call | Strings/RegExp | Loop / virtual | Role |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| `development/moment/dates` | 3 / 3.6504 | 24836 | 26.13% | 19.96% | 17.58% | 9.26% | 37.68% / 0.04% | target |
| `development/diff/compare` | 4 / 3.6178 | 18244 | 31.69% | 23.73% | 26.78% | 8.38% | 32.19% / 0.05% | target |
| `development/meriyah/parse` | 5 / 3.3434 | 7310 | 33.04% | 28.10% | 14.66% | 2.93% | 49.81% / 0.68% | target |
| `development/js-yaml/document` | 1 / 4.3374 | 12432 | 16.02% | 13.11% | 17.29% | 20.83% | 43.43% / 0.25% | control |
| `development/immutable/persistent` | 2 / 4.0096 | 10695 | 15.94% | 11.74% | 28.99% | 1.06% | 47.42% / 0.68% | control |
| `external/jetstream3-js-subset/hash-map` | 7 / 1.5026 | 1489 | 21.69% | 19.01% | 1.07% | 0.00% | 76.02% / 0.00% | control |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 24 / 1.0691 | 2165 | 16.95% | 14.60% | 9.38% | 0.00% | 69.15% / 0.00% | control |
| `sentinel/heterogeneous_property_read` | 1 / 1.4232 | 675 | 16.00% | 16.00% | 0.00% | 0.15% | 76.44% / 0.00% | control |
| `broad/property_read` | 1 / 5.1528 | 1305 | 0.00% | 0.00% | 0.00% | 0.00% | 35.40% / 64.60% | control |

Selected targets are development ranks 3–5: the strongest named owner/envelope
among the five leading development cases sampled here, with Rust/NG gaps above
3.3x. This selects a general ordinary-path mechanism, not a special admission
rule for any library. The loop remains the largest owner; the named envelope
is only evidence of material reach, not a promise that the final gates are easy.

Higher-ranked js-yaml has more string/RegExp ownership (20.83%) than named
ownership (13.11%); immutable has 28.99% Vm/env/call versus 11.74% named. Keep
both as controls rather than crediting string or activation work to M1.
Hash-map has material named reach (19.01% owner) but a smaller 1.50x NG gap;
raytrace has 14.60% named plus 9.28% call and 5.13% other function work.
Both guard accelerated access, construction and class-field costs. The
heterogeneous sentinel actually samples typed execution (65.78% owner) and
literal-data slot reads (16.00%); it remains a mandatory generic control.
Broad/property_read is 64.60% virtual-object ownership (62.07% specifically
`Vm::run_virtual_object_op`) and 35.40% loop ownership; no named-envelope
samples were observed. Its 5.15x NG gap is not grounds to select M1 from this
profile. All 83 non-target policy cases, including all six sentinels and all
other development cases, remain controls; held-out was never inspected.

Top owners and concrete named-path owners, pooled across all windows:

| Target | Largest concrete owners | Concrete named owners |
| --- | --- | --- |
| moment/dates | wide activation `run` 21.06%; native `call_native_family` 6.12%; `Vm::run_current_activation` 6.10% | `own_data_property_read` 2.68%; `op_get_prop_named` 1.95%; `NamedPropertyCache::probe` 1.37% |
| diff/compare | `Vm::run_current_activation` 12.82%; wide activation `run` 12.25%; `apply_env` 2.97% | `object_data_literal` 2.64%; `property_value` 2.15%; `property_value_key_with_receiver` 1.96% |
| meriyah/parse | wide activation `run` 27.55%; `Vm::run_current_activation` 9.99%; `own_data_property_read` 5.18% | `own_data_property_read` 5.18%; `PropertyStorage::insert` 2.59%; `write_existing_own_data_property` 2.08% |

## Draft plan and validator

Plan SHA-256 `584d22cf4638c368d3e61bfb100517c438396a550b2338b6469564a2d9cf4d0f`; unit `interpreter-core-v2-kernel-layout-authority`.
The [review notes](../proposals/T064-successor-review.md) explain stage handoff,
manual checks and freeze prerequisites; the JSON mechanism owns the full
proposed stage/ledger contract and its typed fields bind the checked budget. It uses the proposed structural-prerequisite override,
not an assertion that a neutral K leaf earns normal retention.

K→A→B keeps M fixed. K/A stage cap is 1.025, further limited by each case's
policy band; final target payoff is min(.97, 1/B_i). New workers-4 policy v6,
SHA-256 `be123f7c5cfd46aa863467256eb8bf6947ff64f72541efc371f05a36c5535a31`;
instruction-blind with all three targets declared memory-sensitive, 5 pairs ×
6 layouts, 60 formal blocks and 30-pair held-out acceptance plus required
continuations/panels. Resource report/raw completeness and all existing
instruction/lane/portfolio/held-out gates remain. No numeric RSS ceiling is
invented; review that limitation before freezing.

| Target | B_i | Final candidate/M maximum |
| --- | ---: | ---: |
| `moment/dates` | 1.050127431 | 0.952265383 |
| `diff/compare` | 1.099109904 | 0.909827121 |
| `meriyah/parse` | 1.071894924 | 0.932927265 |

Failure budget: successor K ≤1; A+B share exactly one remaining failure;
successor aggregate ≤2 with no borrowing. Old `0a9af81b` remains the spent
failure of the original account, identified in the [closure draft](../proposals/T064-layout-edges-closure.md).
A valid K failure stops the route and redirects effort to M2. The stricter
per-stage ledger and diagnostic-only stage-band audit are manual contract
obligations, not features of the schema validator.

Validation command (paths relative to workspace root for the external inputs):

```sh
./scripts/performance-decision.sh validate-unit \
  --unit tasks/proposals/T064-kernel-layout-authority.json \
  --queue <workspace>/outputs/imac-bench/20261022T050553Z-t064-successor-exact-profiles/out/queue.json \
  --profile-root <workspace>/outputs/imac-bench/20261022T050553Z-t064-successor-exact-profiles/out/profiles
```

Exit 0: `status=valid`, `priority_mode=override`, `base_sha=9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30`;
queue `f6666a68cb8e1841418d6f5012e6f7810b0c25ebf982b45ef6d5564855e0ae13`; unit `584d22cf4638c368d3e61bfb100517c438396a550b2338b6469564a2d9cf4d0f`.
Validation output SHA-256 `3c2a99a12b9dbfc41fa3aa98027c5ba376d87b0973e8e8fedac6928066e6df95`.
All nine target receipts (three windows each) pass the validator's executable,
queue, opportunity and copied-asset hash checks. The separate 27/27 receipt
verification also covers every control window. Validation verifies bindings
and structure, not the lead's approval, interpretation of profile reach,
manual ledger/static gate or runtime performance.

## Receipt manifest

For an opportunity ID, replace `/` with `--` to obtain its profile directory.
Each row is `out/profiles/<id-with-double-dashes>/<repeat>/receipt/receipt.json`.
`profile` is its copied `profile.sample`; the original `raw.sample` has the
same hash. Receipt/workload assets remain beside the receipt.

| ID | Repeat | Receipt SHA-256 | Profile SHA-256 |
| --- | ---: | --- | --- |
| `development/moment/dates` | 0 | `6802d8b30b2dddfcbefb595d0a4b97d3a9902794170d3645325df75e880b3eac` | `90d7f63f8ee388983f77f35fbafdb6c6f2e11cacd32a8d5e523b6dfc7d79b48d` |
| `development/moment/dates` | 1 | `9001a05770443424201f00c83deab997d5fc534c8dd6b9851a2e770ca5d8d5b5` | `a58a7253bad1b2184154d5cea971d0de6acfd794fe4401c5a80562e312113f53` |
| `development/moment/dates` | 2 | `c24083cb1300f1a814175c48b38413bad61fbfe90fffebb34ce21757ffac60f2` | `9bdfc4bcf4393ccb40dea3cd0def29bf62883df92bb6aa697ca1de8a395dce0c` |
| `development/diff/compare` | 0 | `d06fae470d1a190aaa81199e69113f3575a8075602a31c6cd684fdc570367b73` | `61e41c1d654d5cd39ab4cabb55ca0476899ae32d28af426b67392da9f7824889` |
| `development/diff/compare` | 1 | `030684664ef18a78903c7e470eafd690ae70eb621ee5d3acab0188dd88dc8a45` | `2beac45f9892dcd553bbc912233f043635524a2d2b311e7b15041c6aeecf316f` |
| `development/diff/compare` | 2 | `fc87eeecf035d819d38e91d714672eac2f7e945de004e047c12ef3c65475694a` | `ff91bb75e3260b963394d5a8e76cadbbb8b9b4b470298143beef47c93f3ff1ce` |
| `development/meriyah/parse` | 0 | `c4d2eee2804151f76dc00a81e10792794bca0967d16039d82aa3354047396905` | `3966a914a205b73933d95e8e7be895776a3c2316e4593e7cf0351a4774ee94f5` |
| `development/meriyah/parse` | 1 | `19ccf988fec17697bd5c0adeeaa5276c1c83c2c4332ec233024b1c072ab1a93f` | `71c7dc3590568d8c8b8a7aa6bea53969ab62c11a7cbbbb6d6fb8afb49b1547b6` |
| `development/meriyah/parse` | 2 | `3a640068934f8a83d2bfbf2e7a73b73409d7b8b9e0ff4d60a04d7753f04940ed` | `a2a836a98cae90cbf00573ae4ea1af3f4a49232258f253d413b94e5552818f03` |
| `development/js-yaml/document` | 0 | `f39836a8ad44dbd40e6f39b7c960d9fc685202124dfcdd910fcd06c032061580` | `d437f5c78116aa1db3057c8af520f6f6fe029ed185967ad8f93cc912cb9f2c3c` |
| `development/js-yaml/document` | 1 | `986280c0ed12ad1097034b38aad6b6cabe07171836f6970495fd6bfcabef0be9` | `982f55a987d4e88dd9a8c7f4e65423d1b43ec722069add9c1cd87aa1d2a32108` |
| `development/js-yaml/document` | 2 | `17080dab8d98e2d34f49570121c2ad7b16c64f7d51d699e6456a8624ea61cf46` | `73107af1ae519e426f80ca5c2bb1a5b3b08a09ccbef193785808475a860c223e` |
| `development/immutable/persistent` | 0 | `4bd4be1fe233af02abc63df3ab8d91701ff912796d5f4d69408195da02f87803` | `32a35597a19fa5c8fd96565024c0b304ca78c91cdd51750e17d825e8457a8709` |
| `development/immutable/persistent` | 1 | `5e9e0d1a56bc33ef08eac9c60a079eab72e069296f55c9ca102985183ac64269` | `8b2e45ef2e52807ef97a65fe2a87bc4f5b3a92a52d8b6a3aaaefa164e3cee440` |
| `development/immutable/persistent` | 2 | `4182e437551456dd8f43ee072cdecfd03b62f075e1efe529563d754fb9b0ea16` | `3a923caf533709eaff823eeb20c86d93d22517ff3098441fa50e2f8f2de0d8de` |
| `external/jetstream3-js-subset/hash-map` | 0 | `1144e11e32699ae01dc37af744caa16668c4be9d066c25c85925fbfe8db456f7` | `7676580b6fb1e4f100a20b1e95e40276004103b330c72ff97d0fd7b1c1264e10` |
| `external/jetstream3-js-subset/hash-map` | 1 | `6ef40b6d100fb75a5d839596986e7d2253a2fc4ef8d847cc8ffa844323d6c33b` | `d2d22bd788e5037b7ddece10bc5922fe59ad89ffc01e4262816cb4fd84a832a1` |
| `external/jetstream3-js-subset/hash-map` | 2 | `104a84d8f459dadc6f05c71695bed9aa2aa861bc7fcf8259e69c106f0e138627` | `8612a6dfadc49c6260447e2ac7040a3a9635adaa5c173c278610acaedb4f5273` |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0 | `43bb7a617fdaaa4e3efbfa9be7335926aaee2cfd50ed1d83ccfcde7257657b17` | `c334a6d4d5ce8ea68ca197be052197bd1cb754703f77f1a6e21a6b8404af16cf` |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 1 | `27c7271bc812e83fef7d186431e46ab635919ad5697260039b54b6fe1251a32e` | `75ea2d4014baa89e71280d76619820a84b50c6f27450c146657402f576dbb892` |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 2 | `5b2343fd405d0e56e219fd45e04dc0f4a971447fe4ffa112ce164b0bb03b1303` | `2d0485430660293c093a2e6ac5d7700271c32f6b9315b6f942435179cd44e6ac` |
| `sentinel/heterogeneous_property_read` | 0 | `ffcfa0800660e7f8fd362ec18561642b7204a9c2c11fa6f02379a87c66540b2e` | `a85bd21570a0f41a155bb838239c2e83d169f3902523d5dfff71f6a93be90244` |
| `sentinel/heterogeneous_property_read` | 1 | `a4ba8797fec2f06d2f6233e3b4da5605aabe223a5344249aa8f21d8996018eab` | `0682546017a1af79516800dd3a6306b626cbf68f87c722c9a5403b524a791d38` |
| `sentinel/heterogeneous_property_read` | 2 | `9b36c2af1d456fd4c6c54552cc423276d757958de24e75924b09ed608dc31988` | `a575dd7d9046f0b31414a766d661de18e81c38cdb3343f16fe4c1736e0e73a3b` |
| `broad/property_read` | 0 | `86679ef3fd4fd9b1554adeb04d8518a5dd0b05314da03230c427ce8291c3a0dc` | `dc5bd970a8688f304668ae7fd0e8197bfbdb83c50f39f9e1521ece7b0b6aa097` |
| `broad/property_read` | 1 | `72a0bf9268bba37f7b3d88638ccb6792838ce44d97a526dc4cc481726354aa48` | `906d96a50c7ad2ad394072c963adc47881f7ffa3d2934ece6aec717420658efc` |
| `broad/property_read` | 2 | `58f8fb7169ce5707251be33403ab83a3d7f1c4ffed10577a8735a5211d171e16` | `e6ade3de5d296b890dfa9260acf2a55dc31b58944df12e54f97aba46bf540625` |

Workload asset hashes (identical across each case's three receipts):

| ID | Workload SHA-256 |
| --- | --- |
| `development/moment/dates` | `54b2f358cc54092074a4460f199771508cdc8369170027a47446d7b6761c1090` |
| `development/diff/compare` | `fbb64c6f59b7718700595025d56b2aa6f5413bba10d41f9fca158ac31d6d00eb` |
| `development/meriyah/parse` | `349b451206a4f46670b0ca0f5fd1cc27e459b411806231b5203390d8909c6bd1` |
| `development/js-yaml/document` | `fc1411db299a717af599ac2e308e68018df72c79cca569c9f3e354e40debe9fa` |
| `development/immutable/persistent` | `98aff279f5ab31b94f89cce2dcc94616744c7186b70b2e16466e4640c8fc5ec1` |
| `external/jetstream3-js-subset/hash-map` | `aa98bc1975d8824840df5c31a397b5dab27d514e1854ff5681ceea8ec4bf2c20` |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | `824daa5582289787f6e25a200892a1d6bdfa682afe9ce76a30e520dc7e03528c` |
| `sentinel/heterogeneous_property_read` | `19b1060789fc96ae941c60ae35995c3b185de319acaf9ce53515bfdb824baf7a` |
| `broad/property_read` | `e4c79e512f11bd3716cba0f2a85d650cc120594b6f06370acd92f74952c3248c` |

## Local verification and handoff limits

During drafting at code revision `4fa3f739` (runtime/build identical to M),
`scripts/check.sh` passed all stages, including
2,335 runtime tests, 662 benchmark-tool tests, 35 script tests and all 5,170
curated Test262 cases. The initial run failed only because this worktree had
not initialized `third_party/test262`; exact output included
`error: failed Test262 subset after 11s`. Initialize only that pinned submodule
(commit `4249661388e5d3f92a85186213da140a6481490f`), rerun the subset (5,170/5,170),
and rerun the full gate: exit 0. No pin or third-party content was edited.
Final full-gate log SHA-256
`85211b07d1f6e94fb171635fa1ff46457b62f8fe42fc8ec5a8b1d7e8d3e8a34f`;
the log is local, not retained in Git. Final context, staged fast gate and
whitespace checks passed; no runtime change means
no new focused semantic slice is needed. `target/debug` was deleted after
checks; no local release build, origin push, merge or worktree removal.

The original frozen JSON still hashes to d184dbcc…; the index is corrected to
show its already-spent valid failure, without enacting supersession. The
[T064 resume block](../T064-kernel-isolation.md) points here for lead review.

Open for the lead: approve the prerequisite override and stage caps, accept
the instruction-blind/60-block/resource approach (or define a resource ceiling
before freezing), integrate/review K0's actual checker and separate T056
allocation/startup inputs, then freeze the successor and enact the closure
in one handoff. Exact I profiles and actual K/A/B gates remain future work;
these M samples certify none of them.

Outside this branch's document boundary, `docs/design/property-layouts.md`
still describes T063 as not yet on main; that dependency-status sentence is
stale for M. Its design invariants are linked here, not treated as new
measurement or implementation permission.
