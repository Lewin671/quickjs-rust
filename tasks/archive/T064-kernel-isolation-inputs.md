# T064 kernel-isolation planning inputs: allocation and startup

- Status: complete read-only attribution; no new runtime code or performance build/timing.
- Implementation: none for kernel isolation; proposed architecture is in the [existing core design](../../docs/design/interpreter-core-v2.md#kernel-isolation-prerequisite-k0-before-m1-resumes).
- Verified at:99ce7e91 planning baseline,2026-10-21; all inputs from done0/non-timing job20261021T222209Z-t056-shared-key-qualification.
- Evidence: [complete family/site histogram](../diagnostics/t064-kernel-isolation/allocation-attribution.json), [method](../diagnostics/t064-kernel-isolation/README.md), [original qualified totals](https://github.com/Lewin671/quickjs-rust/blob/99ce7e91982d14c3b2e53b35682dd741cbf7e8b2/tasks/diagnostics/t056-hot-cold/native-summary.json).
- Unresolved: allocation counts/bytes do not price the remaining instructions or wall time; not a new profile of today's main.
- Next action: independent plan review, then exact-main queue/profiles and static prerequisite. No timing5dd3, PMU stays closed.

## Scope and matching

The same cohort contains canonical f58c696e,8f898dd9 and5dd3a810 images with hashes in the histogram.
All roles use the same whole unpack source (hash equality checked), through the same `--raw --interactive` one-line eval wrapper;
empty is `0;`. MallocStackLoggingNoCompact retains every ALLOC, including realloc allocations and subsequently freed blocks.
Class bytes use actual host malloc_good_size, not16B rounding. These are neither live/peak memory nor measured block-usable sizes/latencies.
The existing summaries' totals and each per-stack aggregate reconcile exactly; raw-history and full-stack JSON hashes remain bound.
Names are structural Rust-v0 decodes. Native history retained function stacks, not source-line/offset PCs; “site” means that stack signature plus requested size.
LTO can inline/alias helpers and move an unchanged allocation under a different visible owner: no claim of removal merely because a symbol disappears.
Combine ordinary layout/fields/edges/cache/payload/construction sites to avoid charging their ownership-name movement as a separate optimization.

## unpack call-site families

| Non-overlapping stack family | f58 count / bytes |8f898 count / bytes |5dd3 count / bytes |
| --- | ---: | ---: | ---: |
| atom interning | 0 / 0 | 11211 / 610596 | 6083 / 487492 |
| bytecode/compiler/VM remaining | 1179 / 29823 | 815 / 28104 | 840 / 37689 |
| function remaining | 82676 / 4093946 | 81098 / 3948956 | 81041 / 3942704 |
| function/array named descriptors | 0 / 0 | 96 / 21389 | 93 / 19597 |
| layout text index | 0 / 0 | 938 / 1385712 | 938 / 1385712 |
| object cold data | 36 / 14400 | 36 / 14400 | 44 / 17952 |
| ordinary object/layout payload and construction | 1877 / 2626448 | 4939 / 4359125 | 4902 / 4344761 |
| other/CLI/allocator | 88 / 1792288 | 88 / 1792288 | 88 / 1792288 |
| realm/environment remaining | 411 / 44170 | 388 / 43722 | 388 / 43722 |
| string/regexp remaining | 735226 / 44107087 | 735111 / 44071951 | 735111 / 44071951 |
| **Total** | 821493 / 52708162 | 834720 / 56276243 | 829528 / 56143868 |

## empty call-site families

| Non-overlapping stack family | f58 count / bytes |8f898 count / bytes |5dd3 count / bytes |
| --- | ---: | ---: | ---: |
| atom interning | 0 / 0 | 933 / 86932 | 932 / 86900 |
| bytecode/compiler/VM remaining | 1152 / 27901 | 788 / 26182 | 813 / 35767 |
| function remaining | 3744 / 383512 | 2173 / 238470 | 2116 / 232346 |
| function/array named descriptors | 0 / 0 | 85 / 20004 | 82 / 18212 |
| layout text index | 0 / 0 | 4 / 4096 | 4 / 4096 |
| object cold data | 36 / 14400 | 36 / 14400 | 36 / 14688 |
| ordinary object/layout payload and construction | 1606 / 229992 | 3097 / 608447 | 3060 / 589075 |
| other/CLI/allocator | 68 / 55596 | 68 / 55596 | 68 / 55596 |
| realm/environment remaining | 301 / 37830 | 286 / 37638 | 286 / 37638 |
| string/regexp remaining | 739 / 69574 | 636 / 36966 | 636 / 36966 |
| **Total** | 7646 / 818805 | 8106 / 1128731 | 8033 / 1111284 |

## Concrete growing call-site paths

The families above partition every allocation once; the following are subsets, not additive inclusive envelopes.
`Shape::first_lookups` sits under ordinary named reads/writes (notably `write_existing_own_data_property`):

| Request/site | f58 count / bytes |8f898 count / bytes |5dd3 count / bytes |
| --- | ---: | ---: | ---: |
| index Box32B |0 /0|469 /15008|469 /15008|
| index buckets3216B |0 /0|226 /726816|226 /726816|
| index buckets6416B |0 /0|64 /410624|64 /410624|
| other index buckets416/816/1616B |0 /0|179 /233264|179 /233264|
| **all index requests** |0 /0|938 /1385712|938 /1385712|
| AtomDomain::intern* text24B |0 /0|5381 /129144|257 /6168|
| AtomDomain::intern* entry48B |0 /0|5611 /269328|5611 /269328|
| atom table buckets/other sizes |0 /0|219 /212124|215 /211996|
| **all atom requests** |0 /0|11211 /610596|6083 /487492|

A large atom-miss path is Vm SetProp→try_create_ordinary_own_data_property→ObjectRef::create_absent_own_data_property→
LayoutEdit::append_slow→AtomDomain::intern_key/intern; final shared append/intern_with_owned retains the same semantic authority.
Shape::append_child/append_atom/recode_child and dictionary append allocate nodes, copied Field vectors, append edges and private dictionary buffers;
the combined ordinary-store family adds1732677 requested bytes (8f898−f58), while the index family adds1385712 and atoms610596.
Those three families gross+3728985B/+15211 events, offset by other families−160904B/−1984 events, yielding **+3568081B/+13227 events**.
This identifies the extra allocations by call path rather than assigning them to the few hundred class128/144 requests.

5dd3 removes5192 events/132375 requested bytes versus8f898: atom-family events−5128/bytes−123104 are most of it;
the rest combines bootstrap batching, payload/construction changes and cold-header costs. Entry48B count5611 and all938 index allocations remain.
The shared-key work reduced allocation traffic; it did not remove the whole-program instruction residual.
Main string/regexp sites remain735111 events/44071951B in both candidates versus735226/44107087B at f58.
A visible81474×88B replace-result site moves from native_regexp_prototype_replace to JsString::new in the stack; its combined count is unchanged.
The81474×9B lastIndex/property-name request likewise moves caller frames. Do not count either LTO naming move as allocation removal.

Subtracting each image's matched empty history leaves813847/826614/821495 events and51889357/55147512/55032584B.
Therefore8f898's extra unpack-body work is **+12767 events/+3258155B** after the empty contrast; one bootstrap is only460 extra events/309926B.
The [header-only accounting](https://github.com/Lewin671/quickjs-rust/blob/99ce7e91982d14c3b2e53b35682dd741cbf7e8b2/tasks/diagnostics/t056-hot-cold/header-cold-accounting.json) independently prices only102 hot objects moving classes:
1632B class savings offset by3904B cold-box class growth/new boxes, net+2272B. This cannot explain the full+3.57MB.
No site histogram establishes which allocator/cache mechanism causes wall excess.

## Startup +7%: measured attribution and its limit

Same canonical, three-warmed-sample medians from the cohort above; no new timing or counter-helper call:

| Diagnostic contrast | f58 |8f898 |5dd3 |
| --- | ---: | ---: | ---: |
| whole empty instructions |13855950|14859332|14840576|
| --help instructions |8649234|8679091|8696618|
| empty minus help (proxy only) |5206716|6180241|6143958|

5dd3 whole-empty excess is984626 instructions (+7.1062%); help differs47384, leaving937242 in the empty-minus-help contrast.
This is an algebraic contrast, **not an exact bootstrap instruction decomposition**: help parses/prints differently, and empty includes init/evaluate/drop.
No claim that937242 belongs to a specific allocator/helper is justified by these counters.
The empty allocation table identifies net+309926 requested bytes/+460 events at8f898 and+292479/+387 at5dd3.
At8f898, ordinary-store metadata/construction+378455B, atoms+86932, named tables+20004 and indexes+4096 gross+489487B;
function/VM/string/env site reductions offset179561B. This is the measured bootstrap allocation source, not a price per instruction.

Separate instrumented5dd3 bootstrap events report398 shapes/6163 copied fields, versus the prior8f898-source441/6981;
Object installation391→50 fields and globals935→458 improve, but TypedArray749, Error684 and remaining globals458 persist.
Those phases total1891 of6163 copied fields (30.68%); all intrinsic installers together retain shape/atom/batch construction absent from f58's payload-only authority.
These are source/event counts, not additive inclusive timing or canonical instruction fractions. The remaining canonical +7% is not fully instruction-priced.
Future startup work needs prefix-phase instruction attribution in an isolated diagnostic archive with matched overhead,
then exact uninstrumented whole-empty confirmation; any instrumented prefix/early exit changes codegen and cannot replace canonical evidence.
Target complete descriptor construction and repeated metadata publication where the new profiles support it; do not assume ObjectData shrinking fixes startup.

## Consequence for the prerequisite

Kernel isolation prevents cold/object changes from moving numeric hot blocks; it does not remove these allocations or make the native regexp/string callback path invariant.
Use these sites as later M1 profiling inputs, not the new kernel plan's fresh-main receipts or a forecast of retained gains.
Fresh profiles must verify remaining repeated probes/index construction, dictionary growth and bootstrap construction before selecting a fix.
No new measurement attempt, rule/band/recipe edit, root operation or runtime patch was made to obtain these tables.

## Machine-code coupling input

Canonical8f898 parent image10211f83… and5dd3 candidate26ed840a… share normalized selected numeric bytes,
but the typed WideLoopFrame body's inlined brand checks shorten their encodings (offsets from function entry):

| Parent comparison offset / bytes / displacement | Candidate offset / bytes / displacement | Delta |
| --- | --- | ---: |
|0x838 /8 /0x82(%r13)|0x838 /5 /0x7b(%r13)|−3B|
|0xf92 /7 /0x82(%rcx)|0xf8f /4 /0x7b(%rcx)|−3B|
|0x1ccc /7 /0x83(%rsi)|0x1cc6 /4 /0x7c(%rsi)|−3B|
|0x1ceb /7 /0x83(%rdi)|0x1ce2 /4 /0x7c(%rdi)|−3B|

The first two compare module-namespace brand to0; the latter two compare Symbol primitive brand to1.
ObjectData plus Rc's16B prefix moves these displacements below128, selecting the short x86 displacement encoding.
The Add payload store consequently moves0x29ca→0x29be. Numeric loop counts stay equal; entry pinning and normalized equality miss the drift.
This is the direct codegen/address cause, not a proven predictor/cache/latency mechanism. Source bisection and header subparts independently support the coupling.
The full parent/candidate image hashes are in the local histogram; exact operand/assembly-hash evidence is preserved in the
[original sealed displacement record](https://github.com/Lewin671/quickjs-rust/blob/99ce7e91982d14c3b2e53b35682dd741cbf7e8b2/tasks/diagnostics/t056-hot-cold/displacements.json).
This gives a self-contained negative fixture specification for T064 without depending on unintegrated runtime files on main.
