# T077 S1 review repair verification

Owner branch `agent/t077-s1-property-authority/codex`, repair base `b2daec46`.
The [repair design](../proposals/T077-s1-review-fixes.md) replaces the reviewed
layout cache policy. The [previous receipt](T077-s1-verification.md) remains
revision-bound history. No push or merge is authorized.

## Implementation and focused evidence

Weak vector/hashed transition edges preserve all live children. Children retain
source paths; only append edges support last-key rollback. Prefix shapes share
append-only descriptors and an incremental ID index; branches detach once.
Individual attribute changes and bulk seal/freeze share cached result shapes.
No-op integrity changes copy nothing; preventExtensions changes only its flag.
The keep-alive cache evicts against a 1 MiB incrementally tracked allocation
budget, including shared buffer growth, key text estimates and weak-edge capacity.
Live layouts may exceed this budget; then strong retention drains. Dictionary
width/non-last-delete/edit policy and physical object/store budgets are preserved.
Production atom interning omits the ID directory; symbol identities use NameMap.
ID buckets use high multiplication bits and have a strided-ID regression.

Regression tests cover live fan-out beyond cache capacity, prefix isolation,
branching, descriptor/accessor integrity, linear tail copies, deletion rollback,
long-key/shared-tail retention and unchanged slot/value semantics. Initial full
perf-counter run passed 2,450 tests; combined agents/perf-counters passed 2,463.
The layout increment fast gate passed (default runtime tests, clippy, format,
context and size checks); no focused Test262 filter matched these paths, so
full/NG/exact gates remain required. Final source verification is pending below.
Host ordering, final counters/RSS and complete exit gates are still in progress.

## Explicit host order increment

Nonempty ObjectRef host inputs now use ordered vectors through the existing
OrderedDataPropertyBuilder. Every nonempty caller (including test fixtures) is
ported; empty HashMap entry points stay source-compatible with S2 and reject
nonempty input. The validated separate-permutation adapter remains test-only.
Iterator results now insert value/done, Promise.withResolvers inserts
promise/resolve/reject, and Proxy.revocable inserts proxy/revoke. Numeric keys
still enumerate numerically; duplicate definitions keep the first key position.
A focused host-order unit test and NG fixture cover these behaviors.
RegExp match-array ordering lives in ArrayRef and requires a scope decision.

Layout increment: `4f38ccdf`; pre-commit fast gate passes with 2,421 default
runtime tests. No Test262 path filter matched that increment. Native and
x86_64 agents/perf-counters checks compile the unchanged size assertions.

## Diagnostic counters and memory

Before: `b2daec46`. After engine sources: `46327127`. Native aarch64 macOS,
Rust 1.95.0, `cargo build -p qjs-cli --release --features perf-counters` for both.
No timing or formal performance acceptance is claimed. Each counter/peak run
is one process; live RSS is a separate process, sampled three times with `ps
-o rss= -p PID` immediately after `print("RSS_READY")`, then held by a 1.5 s
Date.now loop. The result array is read again after the loop; suffix127 has
already set its final object to null. RSS is the median of those three samples,
in MiB (2^20 bytes); physical RSS varies with allocator state and host pressure.
The peak and live columns therefore come from different processes. Startup work
is included in all counters. Inputs below fix the scenario sizes and key order.

| Scenario | Misses before → after | Hits before → after | Field copies before → after | Live RSS before → after | Peak RSS before → after |
| --- | ---: | ---: | ---: | ---: | ---: |
| ctors16 | 600 → 600 | 960,128 → 960,128 | 10,637 → 43 | 65.70 → 65.39 | 65.70 → 46.00 |
| ctors20 | 1,200,552 → 612 | 180 → 1,200,120 | 1,210,589 → 43 | 105.19 → 80.30 | 205.12 → 80.28 |
| json20 | 300,549 → 609 | 160 → 300,100 | 310,331 → 43 | 74.56 → 41.48 | 72.81 → 40.69 |
| distinct100 | 20,552 → 20,552 | 160 → 160 | 1,000,589 → 43 | 26.88 → 14.36 | 26.25 → 14.70 |
| suffix127 | 21,307 → 20,677 | 2,519,404 → 2,520,034 | 2,577,752 → 2,519,917 | 41.17 → 5.80 | 41.08 → 5.72 |
| freeze40 | 592 → 593 | 400,120 → 410,119 | 12,811,369 → 83 | 38.00 → 16.78 | 37.98 → 16.75 |
| delete | 20,556 → 554 | 10,156 → 30,158 | 30,502 → 43 | 11.59 → 7.39 | 11.39 → 7.28 |

The distinct-key counter test proves **zero** copied fields for its 20,000 tail
extensions; the 43 remaining fields in CLI scenarios are startup work. The
freeze counter test proves 40 total copies and one integrity node across 1,000
40-key freezes. Last-delete plus rebuild shares the original append shapes.
Suffix127 still branches and copies its 126-key prefix for each unique suffix;
its retained memory is bounded instead of retaining thousands of wide layouts.
That branch work remains O(width), as designed. The diagnostic suffix index
rebuild count rises from 96 to 100,068 (1,424 → 2,480,688 rebuilt entries), because
branch prefix copies rebuild an index when excluding a shared future tail.
This is a remaining branch cost, not a claim of constant-time arbitrary forks.

The strided-ID test places 128 IDs spaced by 256 into 256 starting buckets:
low-bit multiplication gives one starting bucket; high bits give 95. Assertions
require broad dispersion and verify delete/reinsert/rebuild mappings. Production
symbol lookups retain their probe count; this repair removes SipHash per probe,
not the required prototype-level probes. Atom directory insertion is compiled
only for tests; production sweep sizing uses string plus symbol table lengths.
No new dependency, public cross-crate API or unsafe code is introduced.

Counter binary SHA-256:

- before: `f103cd5d6a833cb04f53eeecc6a614232a63ae098aed9400a714d4c4be18038b`.
- after: `5f695dc940456f175fa39ac67cae1859a7fe84a8ea4d8b04ed26994a17421604`.

Reconstruct the exact scripts with the following Python (under the worktree).
The JSON input is generated by the host to keep transient JS string-building
allocations out of the parse/storage measurement; an earlier concatenation-based
probe was confounded by those allocations and is not used in the table.

```python
from pathlib import Path
import json
p = Path("target/s1-scenarios")
p.mkdir(exist_ok=True)
common = "var out=[];"
ctors = common + 'var ctors=[];for(var t=0;t<20;t++)ctors.push(Function("n", "this.k"+t+"=n;this.b=n;this.c=n"));for(var n=0;n<20000;n++)for(var t=0;t<20;t++)out.push(new ctors[t](n));out.length;'
p.joinpath("ctors20.js").write_text(ctors)
p.joinpath("ctors16.js").write_text(ctors.replace("t<20", "t<16"))
records = [{"k"+str(n%20):1,"b":2,"c":3} for n in range(100000)]
text = json.dumps(records, separators=(",", ":"))
p.joinpath("json20.js").write_text("var out=JSON.parse("+json.dumps(text)+");out.length;")
p.joinpath("distinct100.js").write_text(common+'for(var n=0;n<200;n++){var o={};for(var k=0;k<100;k++)o["k"+n+"_"+k]=k;out.push(o);}out.length;')
p.joinpath("suffix127.js").write_text('for(var n=0;n<20000;n++){var o={};for(var k=0;k<126;k++)o["k"+k]=k;o["unique"+n]=n;}o=null;0;')
p.joinpath("freeze40.js").write_text(common+'for(var n=0;n<10000;n++){var o={};for(var k=0;k<40;k++)o["k"+k]=k;Object.freeze(o);out.push(o);}out.length;')
p.joinpath("delete.js").write_text('function C(n){this.tmp=0;delete this.tmp;this.b=n;this.c=n;}'+common+'for(var n=0;n<10000;n++)out.push(new C(n));out.length;')
```

Run each original file with `/usr/bin/time -l <counter-binary> <script>`.
For live RSS, append `;print("RSS_READY");var end=Date.now()+1500;while(Date.now()<end){};out.length;`
(use `0;` instead of `out.length;` for suffix127), sample upon the marker, then
let the process finish. These extra holding-loop counters are excluded.

## ID namespace capacity boundary

Self-review found that high-bit indexing must cap its shift at the 32-bit key
width when a 64-bit host grows a bucket array beyond 2^32. The correction leaves
all diagnostic scenario capacities unchanged. A focused unit test exercises
capacity 2^33 without allocating that table. The initial exact scan was stopped
during queue preparation and has no burndown; the corrected engine gets a new
complete scan. Final verification and refreshed binary identity follow below.

## Corrected-source diagnostic refresh

At `0ab176a5`, all seven fresh counter runs exactly match the corresponding
`46327127` counter columns above (including the documented suffix branch/index
cost). The following RSS samples and peaks are fresh runs of the corrected
source; the previous table remains the immutable prior-revision measurement.
Counter binary SHA-256:
`d8e34e439676905799953a14336ad28896f087386aecad2fa2cf63c6fca9f98e`.

| Scenario | Corrected live RSS MiB | Corrected peak RSS MiB |
| --- | ---: | ---: |
| ctors16 | 65.39 | 65.39 |
| ctors20 | 80.34 | 80.33 |
| json20 | 42.42 | 41.64 |
| distinct100 | 14.73 | 14.34 |
| suffix127 | 4.77 | 5.73 |
| freeze40 | 16.78 | 16.77 |
| delete | 7.41 | 7.33 |

The pre-existing RegExp match-array key-order issue is deferred under the
owner's path boundary to S2's ArrayRef named-store migration. The RegExp caller
already sets index/input/groups/indices in spec order; ArrayRef enumeration
sorts them. Verified example: `Object.keys(/a/d.exec("a")).join(",")` yields
`0,groups,index,indices,input`; NG yields `0,index,input,groups,indices`.
No exclusion or expected failure was added. No S2-owned source was edited.

## Final-source local gates

Executable Rust code frozen at `0ab176a5`; the following working-tree changes
are module-comment clarifications and receipts only. On 2026-10-30:

- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`: pass, including 2,422 default runtime tests, default/agents/perf-counters clippy and feature checks, 753 benchmark-tool tests, 35 script tests, hook isolation, plans/context/size and 5,170/5,170 curated Test262 cases.
- `cargo test -p qjs-runtime --features agents,perf-counters --lib --quiet`: 2,466/2,466 pass.
- `QJS_CLI_BIN="$PWD/target/debug/qjs" ./scripts/compare-qjs.sh`: 259/259 fixtures pass, including the new host ordering fixture.
- `cargo check -p qjs-runtime --target x86_64-apple-darwin --features agents,perf-counters`: pass. Native and x86 assertions keep KeyId 4 B, KeyLease 8 B, ShapeEntry 16 B, NamedStore 48 B, wrapper 56 B, ObjectData 96 B and Value 16 B. No x86 execution or timing claim.
- Branch scope validation from `b2daec46` passes. Forbidden S2 files are unchanged; no new dependency, unsafe code, push or merge.

Final exact scan (pending completion at this receipt update):

```sh
QJS_CLI_BIN="$PWD/target/s1-final-scan-qjs" QJS_AGENTS=1 QJS_CLI_PROFILE=release \
  TEST262_CASE_TIMEOUT_SECONDS=20 TEST262_TIMEOUT_RETRIES=1 TEST262_BASELINE_JOBS=4 \
  TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 \
  ./scripts/find-qjsng-gaps.sh --exact --all --out-dir target/s1-exact-corrected
```

Rust release/agents CLI SHA-256:
`23923fac9ca4748205e61514b295b1aa6b4f9ecfc2b8115bbeec41f03bd9d32d`.
The scan reuses this copied binary, so later default-feature builds cannot
replace it. Compiler: Rust 1.95.0; native aarch64 macOS. Pinned Test262
`4249661388e5d3f92a85186213da140a6481490f`; NG
`f7830186043e4488f2998759d60a514faf07cbc9`. NG runner/CLI/config SHA-256 are
unchanged from the previous S1 receipt and were rechecked for this scan.

## S2 integration API notes

Two private NamedStore interfaces changed with the representation:
`transform_descriptors(frozen: bool)` selects bulk seal/freeze; its ObjectRef
callers pass false/true. The former borrowed `slot_key` API is replaced by
`slot_matches(slot, &KeyLease)`, since a shared append buffer cannot expose an
unguarded reference across reallocation. Both ObjectRef slot consumers are
ported. S2 array/function consumers based on the previous interface must use
these operations when integrating; no S2-owned source was changed here.

## Complete exact result and handoff

The corrected exact driver exited zero. Verified inventory: **53,572 unique
paths**, **10,900 unchanged NG-configured exclusions** (10,621 feature/exclude
records and 279 fixtures), **42,672 configured/eligible/executed Rust cases**.
Rust passes **42,672/42,672**, with zero failures, timeouts, skipped configured
cases or not-run configured cases. NG passes 42,602 and fails the same 70 cases;
zero NG timeouts. Both-pass is 42,602; Rust passes every NG nonpass. All
NG-pass/Rust-fail, timeout and not-run buckets and actionable gaps are zero.
No configured pass is lost relative to the previous S1 receipt. This is complete
configured coverage, not a claim about the 10,900 excluded files.

The unique path inventory and every configured Rust outcome were independently
checked from cases.jsonl before recording; the copied scan binary hash was
rechecked unchanged. Summary and case-result hashes are identical to the prior
S1 scan, establishing identical configured outcome buckets:

- summary.json: `cffdf394963ec66c41d006192593e9975e6b082a55483748550b7a9fb866e80f`.
- cases.jsonl: `2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.

Burndown appended, never replacing a historical record:

```sh
./scripts/test262-burndown.sh --report target/s1-exact-corrected \
  --commit 0ab176a59ed007ce8ba3d5c227f837fd4c6dd00c \
  --source local-exact-release-agents
```

The [ledger](../../docs/conformance/burndown.jsonl) binds these counts to the
scanned executable source. The abandoned preparation-only scan has no entry.
Final receipt, task/index, ledger and module-comment changes alter no executable
Rust code after `0ab176a5`. The final commit hook reruns the staged fast gate.

Repair commits:

- `4f38ccdf`: live transition edges, shared prefix descriptors, byte accounting,
  cached attributes/integrity, last-delete rollback and atom/index repair.
- `46327127`: ordered host construction and all nonempty direct caller ports.
- `d302d43e`: revision-bound diagnostic counters/RSS and reproduction inputs.
- `0ab176a5`: ID-index capacity boundary and focused no-allocation test.

Remaining work: S2 must preserve RegExp match-array named-key insertion order
and adapt the two private NamedStore interfaces above; the lead owns formal
whole-portfolio performance acceptance. Prefix forks retain the documented
O(width) copying/index rebuilding cost. Existing JS value RC cycles remain S6
work; shape source links form an acyclic graph, not a new value owner.
Disk checks remained above the 8 GiB stop threshold (lowest observed sample
about 19 GiB; 21–22 GiB near scan completion). No push, merge, excluded-case
change or S2-owned source edit occurred.
