# T057: Linear non-ASCII literal-search repair

- Status: closed-landed; owner change committed, integration pending.
- Implementation: present; sentinel-free byte search and KMP unit fallback.
- Verified at: the commit adding this record, based on `fa96320f`, 2026-10-20;
  runtime/test source is byte-identical to build snapshot
  `53331cd4d319f26d44fb475d49b422054178c771` (a temporary bench ref).
- Evidence: focused tests, fixture and instruction/codegen tables below.
- Unresolved: arm64 dynamic-property work is +1.07%; whole-portfolio
  regression screen has not run.
- Next action: lead reviews/integrates this branch after its regression screen.

## Scope and invariant

Owner `agent/string-search-linear/codex`; base `fa96320f`; worktree
`worktrees/quickjs-rust/string-search-linear`. Correctness follow-up to
`b419d71e`, not a new stage-2 performance unit. No merge, origin push, timing
job or worktree removal. No dependency, public API, storage or executor change.

Canonical storage means a nonempty pattern without surrogate sentinels cannot
start or end inside a stored scalar: a half-pair would require a sentinel in
the pattern. Valid UTF-8 substring matching cannot begin at a continuation
byte either. Sentinel-free byte matches therefore coincide with unit matches,
even in a sentinel-bearing subject. Forward bounds inside a pair round up;
reverse bounds limit match ends to `position + pattern_unit_length` and round
down. Empty patterns still visit every unit boundary. Actual U+F0000..F07FF
scalars remain escaped and take the unit path, as do lone surrogate patterns.

`indexOf`, `includes`, `lastIndexOf`, literal split and replace/replaceAll
all use the repaired search helpers. Split uses disjoint byte slices;
replacement match offsets use one forward byte/unit cursor. The unit fallback
builds KMP prefix links once per operation and scans disjoint ranges (reverse
search keeps overlapping matches). Both searches are O(n + m), excluding
output size. ASCII call paths are unchanged. Existing replacement assembly
and observable coercion/callback ordering remain in their owning helpers.

## Tests and gates

Focused low-level tests: 7/7; eval-level string search/split/replace tests:
4/4. Unit oracles cross forward/reverse bounds, empty patterns, paired and lone
surrogates and all 2048 real sentinel-range scalars. KMP work-count test uses
100k/200k repeated high units with 10/1000-unit prefixes, forward and reverse;
each scan performs at most 2n equality checks, independent of pattern length.
The new `string-search-linear.js` NG fixture also covers long-prefix failures,
all six APIs, paired boundaries and callback offsets.

| Gate | Result |
| --- | --- |
| `check-touched.sh --staged --explain` | exit 0; 2335 runtime tests, format/clippy/context/size |
| `compare-qjs.sh` with candidate release image | exit 0; 256/256 fixtures |
| `find-qjsng-gaps.sh --filter test/built-ins/String --all` | exit 0; Rust 1221/1221, NG 1216 pass / 5 fail; zero actionable gaps |
| `check.sh` | exit 0; workspace tests (runtime 2335), agents/perf-counters, 662 benchmark-tool and 35 script tests, all plans/context/size/hook/syntax gates, 5170/5170 curated Test262 |

No automatic focused Test262 filter maps these String source paths, so the
exhaustive String slice above ran explicitly. This is a scoped scan, not a
full burndown scan. The five NG failures are its unchanged regexp-prototype
v-flag cases (the same paths recorded in T057/T063). The first String-slice
invocation failed during concurrent reference-engine setup with exact output
`Error: not a CMake build directory (missing CMakeCache.txt)`; after the NG
build completed, the sequential rerun above passed. No source fix was needed.

## Retired instructions (local arm64, diagnostic only)

macOS 26.6.2, rustc 1.95.0, default features (agents off), canonical release
profile: fat LTO / 16 codegen units / abort, checked-in Rust flags.
Base `fa96320f` vs snapshot `53331cd4`; three alternating base/candidate
pairs after warming both images once for each workload. `/usr/bin/time -l`
retired instructions only; discard wall/cycles. Every paired stdout SHA-256
matches. Inputs: `s = 'é' + 'a'.repeat(n)`, `p = 'a'.repeat(m) + 'b'`;
five method calls per process. Run `s.METHOD(p)` for search/split and
`s.METHOD(p, '!')` for replacement. Append `String(result)` to an accumulator
on each call and return it via `qjs --raw -e`. These are whole-process counts,
including string construction/output; they are not isolated matcher counts.

| n | m | Method | Base median | Candidate median |
| ---: | ---: | --- | ---: | ---: |
| 200,000 | 10 | indexOf | 133,788,559 | 48,224,840 |
| 200,000 | 10 | lastIndexOf | 143,359,263 | 64,432,515 |
| 200,000 | 10 | includes | 133,459,238 | 48,111,924 |
| 200,000 | 10 | split | 253,553,545 | 147,722,409 |
| 200,000 | 10 | replace | 317,232,504 | 238,076,645 |
| 200,000 | 10 | replaceAll | 317,000,614 | 238,000,597 |
| 200,000 | 1000 | indexOf | 1,120,624,323 | 48,536,826 |
| 200,000 | 1000 | lastIndexOf | 1,130,655,974 | 62,443,206 |
| 200,000 | 1000 | includes | 1,121,180,179 | 48,374,018 |
| 200,000 | 1000 | split | 1,241,493,291 | 148,043,203 |
| 200,000 | 1000 | replace | 1,303,458,487 | 237,724,138 |
| 200,000 | 1000 | replaceAll | 1,304,615,718 | 238,048,334 |
| 400,000 | 10 | indexOf | 245,877,489 | 74,891,621 |
| 400,000 | 10 | lastIndexOf | 265,513,587 | 106,935,828 |
| 400,000 | 10 | includes | 245,190,289 | 74,872,997 |
| 400,000 | 10 | split | 487,977,381 | 275,116,287 |
| 400,000 | 10 | replace | 613,337,783 | 455,284,388 |
| 400,000 | 10 | replaceAll | 612,479,519 | 456,622,622 |
| 400,000 | 1000 | indexOf | 2,225,190,717 | 75,101,643 |
| 400,000 | 1000 | lastIndexOf | 2,245,875,878 | 102,927,360 |
| 400,000 | 1000 | includes | 2,229,433,724 | 75,571,164 |
| 400,000 | 1000 | split | 2,472,016,122 | 276,451,490 |
| 400,000 | 1000 | replace | 2,594,311,608 | 456,059,795 |
| 400,000 | 1000 | replaceAll | 2,594,257,564 | 456,758,094 |

At n=200k, increasing m from 10 to 1000 takes indexOf from 133.8M to 1120.6M
instructions on base, versus 48.2M to 48.5M on candidate. Doubling n to 400k
raises the long-pattern candidate to 75.1M, consistent with linear work plus
startup. Other APIs likewise lose their multiplicative pattern-length cost.

Local image SHA-256:
base `454d4d9948581d69a5026cfd0ba737f67fcc025126c45d69288c99fbe89b4348`;
candidate `cbc91a48a7036634b2c7dc07922c2f70a08ffa1304cc37fe17b0880dbbce39a1`.
Raw local `target/string-search-linear/` artifacts are not retained; counts
and protocol above are the durable evidence. Instruction JSON SHA-256
`444ffad3a4ef12090685e1698d4bc8c63a2c7784d0c1fd053ccff01f0e8d459f`.

## Codegen and unrelated work controls

`compact_fn::wide::activation::run` address-sorted text span, measured with
`tools.benchmark.layout_tables.text_symbols` (unique raw v0 symbol):
local arm64 **18,384 -> 18,768 bytes (+384)**. Growth alone establishes no
extra executed work; instruction controls below diagnose that separately.

x86 build-only job `20261020T125302Z-string-search-linear-codegen` was
submitted only after `imac-bench.sh status` showed no running/queued work.
It uses exact source revisions, canonical triage release recipe with agents
on, `imac-bench.sh run` without `--timing`; no compare/loop job submitted.
Job exit 0; x86 wide run **24,960 -> 24,640 bytes (-320)**. The images
were fetched and decoded with local `nm`/the same extent parser (the iMac's
old `nm` does not demangle Rust names). Image SHA-256:
base `41a863208ffd3427bcb4a6788cf1d0a00f2e5702c46fa0f37c621fd255de45ee`;
candidate `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb`.

Arm64 instruction controls use three warmed alternating pairs on the same
images. Broad cases count the 2M-to-4M iteration increment; recursive uses
65536-to-131072 (127 calls per iteration). Workload files are the committed
`benchmarks/workloads/{broad-micro,generic-sentinels}.js`; pass case ID and
iteration count to `qjs --raw FILE CASE N`. All paired stdout hashes match.

| Control | Base median increment | Candidate median increment | Ratio |
| --- | ---: | ---: | ---: |
| string_slice | 3,703,994,285 | 3,701,550,753 | 0.999340 |
| property_dynamic_read | 3,415,472,703 | 3,452,070,297 | 1.010715 |
| recursive_call_tree | 2,877,846,797 | 2,884,988,817 | 1.002482 |

The dynamic-property instruction increase is about 1.07%, slightly above the
proposed 1% work threshold; do not label this arm64 build neutral. The wide
body also changes stack layout (base reserves 0x400 bytes below saved
registers; candidate 0x3f0). No exact spill/call-path attribution is established.
This arm64 observation is a risk for the lead's screen, not evidence about
x86 instructions or latency. No x86 instruction probe was submitted after
another host job started; its codegen sizes above remain valid.

## Proposed regression screen (for the lead; not an acceptance result)

Correctness/complexity repair: one undisturbed 30-block workers-4
three-engine comparison against `fa96320f`, exact committed candidate.
Each broad/sentinel/external/development lane geomean must be <=1.010;
no case's 95% interval lower bound may exceed 1.030. Any failed case/lane
gets no-timing instruction follow-up on the exact comparison binaries:
ratio <=1.010 identifies a placement effect and does not block; above 1.010
requires fixing added work and repeating the comparison. Freeze the rule
before collecting that screen. This follows T057's prior correctness screen;
the owner makes no latency or whole-portfolio neutrality claim.

## Regression screen (frozen by the lead, 2026-10-20, before measuring)

Same rule as the T057 correctness screen: one undisturbed 30-block workers-4
three-engine comparison, base `fa96320f`, candidate `9578dcee897b`. Each
broad/sentinel/external/development lane geomean at most 1.010; no case's 95%
interval lower bound above 1.030. A failed case or lane gets a no-timing
instruction follow-up on the exact compare binaries: ratio at most 1.010 is a
placement effect and does not block; above it, the added work is fixed and the
comparison repeated. The arm64 +1.07% `property_dynamic_read` instruction
note is diagnostic only; x86_64 instructions decide.

Result (lead, 2026-10-20): pass. Formal `20261020T130140Z-search-linear-formal30`
(base `fa96320f`, candidate `9578dcee`), exit 0 after 2199 s, not DISTURBED.
Lane geomeans: broad 0.9960, sentinel 1.0000, external 1.0001 (45),
development 0.9949 (10). Two cases exceeded the interval rule: broad
`empty_loop` 1.0459 (lower 1.0432) and external `kraken-1.1/imaging-gaussian-blur`
1.0522 (lower 1.0452). No-timing follow-up `20261020T142221Z-search-linear-instr-followup`
on the exact compare binaries (`41a86320...` base, `f5822aa3...` candidate):
empty_loop 107.02 vs 107.03 instructions per iteration (1.0001); gaussian-blur
whole run, median of three, 123.916e9 vs 123.927e9 (1.0001). Both are placement
and do not block.
