# T077 S1 index-proof implementation milestone

Source base `ace5f2ad`, branch `agent/t077-s1-property-authority/codex`; not S1
acceptance. [Foundation inventory](T077-s1-foundation.md) owns the construction
and mutation inventory. Ordinary receivers still use only the original store;
this milestone does not introduce a production dual authority or fallback.

## What changed and why

Review found that the foundation's standard HashMap insertion could search the
ID index after the counted lookup. The new local `IdIndex` consumes a scoped
vacant-bucket proof. A cloned immutable layout preserves its bucket positions;
no second operation lookup is needed. Insertions reserve capacity for the next
lookup; cold rebuilds are counted separately, including all reinserted entries.
No dependency, unsafe block or crate is added. This repairs the implementation
of the approved one-probe contract; it is not a design correction or S2 API change.

Dictionary deletes swap the last live slot into the removed slot and retain
explicit insertion-order numbers. This avoids a full vector shift and keeps
zero order tombstones. The ID index compacts its own tombstones, and probes
always retain empty terminators. Accessor slot relocation is atomic under the
same borrow. Order exhaustion compacts live order numbers; structural epoch
exhaustion publishes a new layout identity before restarting the version.
The perf-counters cache-node gauge includes bounded retained roots/nodes and
releases them on owner drop even when escaping objects retain live layouts.

Type budgets remain KeyId 4 B, KeyLease 8 B, ShapeEntry 16 B, NamedStore 48 B,
RefCell<NamedStore> 56 B on this Mac (aarch64); x86_64 remains unverified.
Old PropertyStorage forms, DynamicPropertyStorage, literal payload lookup and
ordinary cold symbols remain untouched: **zero replaced-store deletions** so far.
No iMac instruction/triage/timing job or formal comparison was submitted.

## Verification

`TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`
passed on `ace5f2ad`: all three clippy configurations, feature checks, 2,399
runtime tests, tooling/script/context/file-size gates and 5,170 Test262 subset
cases. This does not establish complete configured exact-scan parity.

The index working tree passed combined-feature clippy and 23 layout tests
before the final retention-gauge discriminator. Tests force colliding IDs,
10,000 distinct short-lived keys, dictionary growth, accessor-slot movement,
order/epoch exhaustion, >5,000 live shapes, owner/domain teardown and foreign
IDs. Counter checks cover append hits and dictionary insertion with an index
rebuild: one operation ID probe, zero key interns and zero shape-field copies.
Final index milestone source tree, 2026-10-29:

- `cargo test -p qjs-runtime --features agents,perf-counters value::layout --lib --quiet`: 24/24 pass, including the retention gauge.
- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`: passes; 2,404 runtime tests, agents/perf-counters feature checks, three clippy configurations, tooling/script/context/file-size checks, 5,170/5,170 Test262 subset cases.
- `./scripts/compare-qjs.sh`: 257/257 fixtures pass against the pinned reference.
- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check-touched.sh --staged --explain`: passes; 2,404 runtime tests; no focused Test262 filter matched these foundation paths.
- `git diff --check` and `./scripts/check-context.py`: pass after final documentation update.

The source implementation is the index milestone commit following `ace5f2ad`;
T077's resume block binds its hash after commit. No claim about S1 acceptance.

The worktree initialized only the pinned top-level QuickJS-NG reference;
its nested submodules and the main checkout are untouched. Shared Test262
remains read-only through TEST262_DIR. Complete exact scan/burndown is deferred
to the ordinary-store cutover candidate; no partial scan or record was created.
