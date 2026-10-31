# T077 S2 round-1 verification and handoff

Engine source: `e1d8a6a3`; branch `agent/t077-s2-named-stores/codex`.
This receipt covers the lead's A-F feedback after `d38f30a7`. No origin push or
main merge. Temporary iMac refs were authorized measurement transport.

## Implemented decisions

S1 tip `9923098e` is merged at `885e0e31`, including live transition edges,
byte-bounded retention, append-shared tails, bulk integrity transitions,
delete-last rollback, atom hashing/ID high bits and ordered construction.
RegExp match-array ordering has an NG fixture, including the d-flag indices tail.

One NamedStore owns all named descriptors and the actual prototype edge. Its
layout includes weak prototype identity/flavor, avoiding retained shapes pinning
JavaScript prototypes/realms. Shared guards compare allocation identity;
dictionaries alone compare versions. Used prototypes have private layouts and
one validity state, with uncapped weak users and iterative invalidation. Ordinary
transitions invalidate nothing. Explicit realm inputs replace implicit Function/
Array binding lookups. Pure data/accessor/missing outcomes share that proof.

Arrays/functions have lazy authorities: probes allocate no array cold storage
or unrelated function defaults. Verification compares complete pure outcomes
and fails on exotic/accessor mismatches. Dead guards are stale; Object.create
uses live array/Map/Set identity. Switches are captured once per domain. Reads
never intern; empty native contexts share with COW. Shape matches create no Weak.

General payoff repairs keep the same authority: small data-hit ABI, borrowed
first-prototype loads, parallel shared-own guards, numeric live-slot reads,
computed-string lease metadata, direct creation roots/identity literal maps and
weak static-name links. These use operations/types/data flow, not benchmark
source/bytecode templates. Rationale: [correction plan](../proposals/T077-s2-prototype-layout.md).

E fixes have five NG fixtures: global var synchronization (named/computed typed
stores deopt before effects), internal StringData slot, Map/Set prototype
identity, typed-array canonical numeric barriers and iterative release of
200,000 array/function named links. A retained middle link stays valid.

## Commits

- `885e0e31` Merge S1 review repairs into the S2 named-store authority.
- `c2d65271` Keep named reads pure and share empty native contexts.
- `9fab7b0c` Guard named ICs with prototype-sensitive layouts and validity generations.
- `26fee930` Preserve internal slots and exotic prototype semantics in named operations.
- `dbe43671` Keep data IC hits small and reuse computed string key leases.
- `63d30d83` Borrow first-prototype holder slots on named cache hits.
- `903a5dc0` Compare shared own-data IC guards in parallel.
- `61966610` Load guarded numeric property slots without Value transport.
- `3feb4a1d` Seed creation layouts once with explicit realm prototypes.
- `e1d8a6a3` Link static miss-path names through weak key leases.
- `3cba3b6c` Freeze S2 round-one source for final conformance verification.
- `b091f9e2` Record counters and iMac diagnostic outcomes.

## Verification

All runtime/build/harness inputs after the engine source above remain identical;
[artifact/source pins](T077-s2-round1-source.md) bind both production targets.

| check | outcome |
| --- | --- |
| Full gate, `./scripts/check.sh` | pass: 2,439 runtime; 5,170 curated; workspace/agents/counter/clippy/tooling/context gates |
| `QJS_VERIFY_IC=1 cargo test -p qjs-runtime --lib --features agents,perf-counters --quiet` | 2,486 pass |
| x86 same runtime features | 2,486 pass, job 20261030T160533Z-t077-s2-r1-final-x86-rss |
| `./scripts/compare-qjs.sh`, default/off/verify | 266/266 each |
| IC-off semantic runtime tests | 2,420 pass; 19 cache-admission proof tests skipped, since they assert enabled IC state |
| Review t/ replay | 31/31 identical on/off/verify outcomes |
| Instantiated review chain fuzzer | 300 seeds, 900 successful processes, on/off/verify identical |

The t/ comparison is 29/31 equal to pinned NG. fn01_order differs in builtin
own-key ordering/extensions, also observed at dd0905b2. ic06_ta differs on NG's
inherited NaN behavior; Rust applies the canonical numeric barrier to NaN too.
The requested -0/Infinity/OOB/fractional differential fixture passes. These
reference differences are not described as stale IC hits or silently hidden.

### Complete configured exact scans

All three reports contain 53,572 unique case records, 42,672 configured Rust
passes, zero failures/timeouts/not-run cases, and 10,900 identical NG-configured
exclusions. NG passes 42,602 and fails 70; every NG pass is a Rust pass.

| engine source / ledger commit | mode | Rust passes | gaps | ledger source label |
| --- | --- | ---: | ---: | --- |
| 26fee930 / 26fee930 | earlier cached checkpoint | 42,672 | 0 | local-exact-release-agents-s2-r1-cached |
| e1d8a6a3 / e1d8a6a3 | final cached | 42,672 | 0 | local-exact-release-agents-s2-r1-final-cached |
| e1d8a6a3 / 3cba3b6c | final IC-off | 42,672 | 0 | local-exact-release-agents-s2-r1-final-ic-off |

All three cases.jsonl files have SHA-256
`2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.
Pass/exclusion sets are identical: zero lost passes, no new exclusions, and
cached/off outcomes match. Both final exact commands exit 0. The pinned release
CLI remains byte-identical to the source checkpoint after both runs.


Commands (both use the pinned agents release CLI, 20 s per case, four workers):

```sh
QJS_CLI_BIN=<pinned-cli> QJS_AGENTS=1 QJS_CLI_PROFILE=release \
  TEST262_CASE_TIMEOUT_SECONDS=20 TEST262_BASELINE_JOBS=4 \
  ./scripts/find-qjsng-gaps.sh --exact --all --out-dir <report>
# Repeat with QJS_DISABLE_IC=1 for the full IC-off comparison.
./scripts/test262-burndown.sh --report <report> --commit <source> --source <label>
```

The IC-off ledger uses documentation-only descendant `3cba3b6c`, whose runtime,
CLI, harness, configuration and corpus inputs match e1d8a6a3. Each completed scan
has one immutable ledger entry; no partial scan is recorded. Intermediate scans
stopped after source supersession are not complete evidence. No exclusions,
allowlists or expected failures changed.

## Payoff and limits

[All seven iMac cohorts](T077-s2-round1-measurements.md) retain automatic
**likely loss**. The final cohort `20261030T155647Z-t077-s2-r1-static-triage` gives
point estimates versus dd0905b2: development 0.998020; heterogeneous 0.6571,
prototype 0.9954, churn 0.8926, dynamic 0.7956. These satisfy the requested
narrow diagnostic point checks; intervals overlap 1.0 in some cases, and broader
controls still regress. No formal four-aggregate acceptance or promotion is claimed.

[Final counters](T077-s2-round1-final-counters.md) record all ten development
checksums and all receiver families/review scenarios. Weak static-name links
remove repeated recovery at megamorphic sites without pinning keys. Stable
600-site class reads reach 11,400/12,000 semantic hits; cold miss attempts are
reported separately. Array/function 300k-receiver scenarios retain 299,999 hits.

Payloads: NamedStore 56 B (RefCell 64), ObjectData 80, ArrayData 88 (cold 32),
FunctionData 304, IC entry 32, handle 8, debug state 88. Value/JsString handles
stay 16/8. String metadata is 80 B on arm64, including an 8-byte lease memo.
Assertions run on arm64/x86_64; detailed target/RSS evidence is in measurements.

Arrays retain more RSS than base because their explicit header is larger.
The final one-pair diagnostic has called/no-call array peaks within 16 KiB and
no extra called-function peak; this proves no formal resource gate. It records
the array RSS regression, not a waiver. Lead-owned formal aggregate/held-out
and >=15-pair resource collection remain necessary before accepting S1+S2.

## Deletions and scope

Round-1 diff from d38f30a7 through e1d8a6a3 removes 1,973 runtime source lines
and adds 5,192 across 132 files, including mechanical realm API ports and the
requested S1 merge. These counts are diff churn, not a claim that every removed
line was an obsolete mechanism. Original S2 table/cache deletions remain in the
[initial receipt](T077-s2-verification.md).

| round-1 owning path | removed lines in diff | replaced mechanism |
| --- | ---: | --- |
| bytecode/named_property_cache.rs | 253 | subscription/first-edge/descriptor hit routing |
| value/layout/version.rs | 40 | shared epochs and capped subscriber lists |
| value/array.rs | 212 | cold named authority, implicit prototype bridge, repeated creation roots |
| function/value.rs + named.rs | 68 | old cold prototype/default probe paths |
| value/object.rs | 125 | separate prototype cell and allocated contextual array identity |
| property/prototype.rs | 34 | mutable-binding intrinsic resolution |

No new crate/dependency/unsafe/FFI/Rust async, parser shortcut or environment
snapshot. The next useful action is lead formal acceptance, then diagnosis of
remaining control/construction losses if that acceptance fails. This owner has
not pushed origin or merged main.
