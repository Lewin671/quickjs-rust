# T077 S1 explicit-owner preparation

Source parent `4e464142`, branch `agent/t077-s1-property-authority/codex`.
This is preparation for the single ordinary string/symbol cutover, not S1 exit
or a second production property authority. The [error-boundary proposal](../proposals/T077-s1-fallibility.md)
owns the pending lead decision; T077's resume block owns current status.

## Implementation

Ordinary constructors and OrderedDataPropertyBuilder now require an explicit
AtomDomain, including null/no-prototype creation. Root script/module setup
creates its domain before globalThis and intrinsics and shares it into the
realm. RealmState owns PropertyOwner outside the domain, retaining the bounded
shape root. Context-bearing creation paths pass their realm domain; detached
helper records receive a domain or a live owning object explicitly. No live
object graph is cloned or relinked.

All 220 inventoried ordinary constructor/builder calls were migrated. Native
function construction also receives its domain (178 inventoried calls); the
function retains it in NativeContextData, alongside its pre-existing native
captures. User functions continue to use their creation realm and indexed
upvalues; no binding snapshot is introduced. Bound functions receive an explicit
domain. Nonconstructable natives and TDZ sentinels no longer allocate the unused
ordinary prototype that the old shared constructor created and immediately
released. NativeContext keeps its HashMap buckets unallocated until capture
insertion; native-domain metadata itself now has an owning Rc payload.

ObjectData temporarily retains an 8-byte domain field while its old store is
still authoritative. The cutover must remove that field and retain the domain
through Layout, not leave a compatibility payload. The temporary ObjectData
size guard is 120 B on this Mac; FunctionData remains within its 296 B guard.
No final ordinary-hot-payload or x86_64 size acceptance is claimed.

Function representation tests moved to `function/value/tests.rs`, and unchanged
URI algorithms moved to `global/uri.rs`, to keep first-party source below the
2,000-line limit after signature/call-site expansion. No dependency or unsafe
code was added. Old-store deletion count remains zero; ordinary symbols are
still in the original cold table until the single cutover.

## Verification

- `cargo test -p qjs-runtime --features agents,perf-counters --no-run --message-format=short`: compiles the owner-port tree.
- `cargo clippy --workspace --all-targets -- -D warnings`: passes.
- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`: passes; 2,406 runtime tests, agents/perf-counters checks and clippy, 753 benchmark-tool tests, 35 script tests, remaining gates and 5,170/5,170 Test262 subset cases.
- `./scripts/compare-qjs.sh`: 257/257 fixtures pass.
- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check-touched.sh --staged --explain`: passes; 2,406 runtime tests and 117 selected Test262 cases; format/clippy/context/file-size checks pass.

New discriminators cover native-domain retention/release without a dummy
prototype and read-only key lookup after ID exhaustion. No complete exact scan
was run and no partial burndown entry was written. No iMac job, timing result,
formal neutrality evidence, push or merge. Disk remained above 37 GiB free.
