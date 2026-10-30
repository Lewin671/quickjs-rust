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
