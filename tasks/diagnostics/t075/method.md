# T075 collection and source-map audit

Measured engine source: `59eb72e511cca013cf59d6736922c9c666cb883f`, 2026-10-23 UTC
(2026-10-24 CST). Interpretation: [T075](../../archive/T075-executor-anatomy.md).
No runtime, dependency, benchmark, compiler-policy or held-out change.

## Images and address collection

The canonical image comes from `scripts/lib.sh::qjs_build_revision`, a clean
commit worktree and the existing `build_cache` recipe: release opt-level 3,
fat LTO, 16 CGUs, abort, agents, incremental off, v0 names, function alignment 4.
iMac: x86_64 macOS 12.6, rustc 1.95.0 / LLVM 22.1.2. NG is the pinned
`f7830186043e4488f2998759d60a514faf07cbc9` Release image. Binary hashes:

- Rust: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- NG: `919f7235f61412ba69faf6aa05b534869aa7d69d1583b3d7aab3cd5b67cd6f40`.

Only `screen.resolve_cases(['development'], ...)` resolves inputs: ten pinned
npm programs, no amplification. Each runs three times on canonical Rust,
full-debug Rust and NG, alternating role order. Sampler targets the engine PID:
`sample PID 60 1 -mayDie -nodsyms -file raw.sample`. All 90 process and sampler
exits are zero, scripts finish before sampler return, and all stdout checksums
pass. [Invocations](invocations.json) retain PID, argv, commands, workload,
stdout and raw-profile hashes. Every raw hash was read back and verified.

Ordinary `sample` merges multiple PCs of a symbol into `+ a,b,...`, losing the
count at each PC. Stripping alone fails: CoreSymbolication finds the original
image by UUID. The final [collector](collect.py) strips copies and replaces
`LC_UUID`; every section in `__TEXT` retains **identical address, length and
SHA-256** before/after. No executable instruction or constant section changes.
The whole file differs in symbol/debug/link-edit/header metadata. No code is
instrumented, relinked, relocated or made ASLR-off for sampling. x86 images
have no code-signature load command requiring a signing mutation.

Final raw engine frames are individual `??? [PC]` entries; **zero engine self
samples have merged addresses**. System-library frames may still be merged.
ASLR normalization is `PC - report_load_address + 0x100000000`. Original `nm -n`
intervals recover functions; repository Rust-v0 decoding recovers their names.
Only the first main-thread tree is used. Node self = count minus immediate
children; nonnegative self counts and exact root conservation are asserted.
The same rules handle recursion without charging caller-inclusive time twice.

## Debug variants and canonical mapping

A normal Cargo `debug=2, split-debuginfo=packed` build changes crate hashes and
breaks raw-name hot-function ordering. It is a retained pilot, **not the source
of the final breakdown**. A second iMac build keeps Cargo's canonical profile,
metadata and flags, with a `RUSTC_WRAPPER` appending only:

```text
-Cdebuginfo=2 -Cstrip=none -Csplit-debuginfo=packed
```

Even that changes codegen: wide grows 24,928 → 25,280 bytes (5,181 → 5,263
executable listing instructions); VM grows 9,712 → 10,096 bytes. Its separate
30 profiles are supplemental and never substituted for canonical counts.
[Full-debug audit](full-debug-audit.json) retains every executor comparison.
Use rustc's original packed dSYM under `build/.../release/deps/`; running
`dsymutil` later on the copied executable cannot find its deleted fat-LTO
object and produces an incomplete dSYM. That warning/pilot is retained.

During the iMac no-start window, local arm64 rustc 1.95.0 cross-compiles the
same unchanged source for `x86_64-apple-darwin`, using release defaults and a
wrapper appending:

```text
-Cdebuginfo=line-tables-only -Cstrip=none -Csplit-debuginfo=packed
```

The target standard library was installed with `rustup target add
x86_64-apple-darwin`; no repository dependency is added. This image is **never
timed or used for sample counts**. Its crate hashes, function placement,
relocations and whole-image bytes differ from the iMac canonical image.

[Analyzer](analyze.py) compares every instruction of each selected function:
remove only `LC_DATA_IN_CODE` ranges (embedded jump tables), listing comments,
RIP displacements, and absolute relocation addresses. Preserve instruction
offsets, mnemonics, registers, non-address immediates, local branch offsets,
and external callee symbol plus offset. Data-table bytes and referenced
literal/pointer bytes are not claimed identical by this comparison.
All **six Rust executor instruction sequences match**, including offsets;
all sampled PCs land on mapped instruction boundaries. It is safe to transfer
those instruction offsets to the line-table image for DWARF lookup. This is
relocation-normalized instruction identity, **not byte-identical binaries**.
The line tables retain inline call chains as well as source lines.

NG is rebuilt out of tree with the canonical Release CMake settings plus
`-g` (`-O3 -DNDEBUG -g`); no third-party source is edited. `JS_CallInternal`
has the same address and size, and its 18,302 non-data listing instructions
match under the same normalization. [Identity](identity.json) and
[seven-function audit](codegen-audit.json) bind all images, sections and dSYMs.
A mismatch remains unmapped rather than interpolated; none occurs in final data.

## Attribution and denominators

[Summarizer](summarize.py) is the exact ordered semantic rule implementation.
It first identifies physical RC instructions, opcode fetch/PC checks/dispatch,
operand bounds checks, Value replace/write traffic, and primitive tag/clone
selection; then assigns named-property, indexed-access, binding, call,
completion, typed-guard and arithmetic contexts. Remaining ambiguous contexts
stay `other`. Inline frames select context; assembly distinguishes actual
compare/conditional-branch bounds checks from address calculations.

Each executor self sample gets **one** category. Categories include required
work and unavoidable checks; they are not independent removable costs.
Transport includes `mem::replace`, pointer writes, register/stack publication,
addressing and clearing. Calls include admitted-frame/window preparation and
returns; completion includes successful `Result` checks, not only throws.
Indexed-access includes its array/index guards. Typed-guards is the separate
entry/residency/explicit-guard family, not all guards in the engine.
Line-zero and incomplete semantic contexts are retained, never called dispatch
by default. Source placement is a compiler annotation, not a causal stall label.

For each program pool the three repeats; divide category self counts by that
program's main-thread N; average the ten percentages equally. Executor-relative
shares divide those mean shares by their sum (39.312655%). Pooled sample counts
are also retained, but do not weight the aggregate toward moment. The keys
`main_thread_percent` mean **sampled main-thread self time**, not wall time.

[Owner-envelope](envelope.py) separately reapplies the frozen T068 v2 ordered
symbol classifier to the very same canonical trees. It resolves original names
at every engine frame and walks callers for transparent library helpers. Thus
its 46.403618% loop bucket and this study's 39.312655% executor subset share a
denominator. The remaining 7.090963% consists of visible helpers, not unpriced
executor instructions. [Owner counts](owner-envelope.json) retain every repeat.

Time-based OS PC sampling is not retired-instruction sampling. No category
instruction count, CPI, causal hardware stall, new wall ratio, formal acceptance
or complete startup profile is inferred. Attachment misses the earliest work;
short cases and source-line-zero annotations limit resolution. `sample` changes
scheduling, and no-timing queue jobs have no quiet-host disturbance monitor.
The full-debug check and repeat ranges support attribution stability, not a
formal confidence interval. T073's address-dependent CPU pathology remains a
risk for every codegen-changing proposal.

## Queue and retained evidence

Only `imac-bench.sh run -- '<one command using "$BENCH_OUT">'` starts remote
work; heredoc commands are transported as one literal argument. Direct SSH only
checks the clock. First clock: 02:31:47 CST. Latest job starts 02:43:29, before
02:45; sampling ends 02:47:55. No job starts during 02:45–05:00. No origin push,
merge, other-owner cancellation or runtime edit occurs. Private queue ref
transport is the wrapper's required mechanism, not origin publication.

[Job ledger](jobs.json) retains command/log hashes for all nine jobs:

| Suffix after `20261023T` | Outcome |
| --- | --- |
| `183202Z-t075-capabilities` | exit 0, tools/disk/clock inventory |
| `183230Z-t075-sampler-options` | exit 127: `rg` absent from queue PATH during strings filtering; later inspection uses available tools |
| `183253Z-t075-build-and-pilot` | exit 0 / 206 s; canonical and ordinary debug builds; unsupported `-noSymbolication` pilot retained |
| `183500Z-t075-address-profiles` | exit 1: transport quoting, no profiles |
| `183602Z-t075-ng-debug` | exit 0; out-of-tree NG debug build |
| `183647Z-t075-address-profiles-v2` | exit 1: workload output directory missing, no profiles |
| `183715Z-t075-address-profiles-v3` | exit 0 / 266 s; 90 symbol-merged pilot profiles, excluded from address counts |
| `183753Z-t075-metadata-preserving-debug` | exit 0 / 107 s; full-debug variant and retained dSYM warning |
| `183840Z-t075-unmerged-addresses` | exit 0 / 266 s; final 90 exact-PC profiles |

External raw inputs live at `outputs/imac-bench/<job>/` and on the queue;
these paths are not assumed durable. Repository evidence is the actual
[counts](counts.csv), [PC map](pc-map.csv), [frame dictionary](frames.json),
[profile denominators](profiles.json), [summary](summary.json), invocation
hashes, build identity, audits and reports. No executable/build output is committed.

`python3 tasks/diagnostics/t075/verify.py` independently recomputes retained
categories and conservation without external artifacts. To remap raw profiles,
run `analyze.py PROFILE_OUT DEBUG_BINARY DEBUG_DWARF NG_BINARY NG_DWARF OUTPUT
LINES_BINARY LINES_DWARF` with LLVM tools on PATH and repository PYTHONPATH;
`summarize.py OUTPUT PROFILE_OUT EVIDENCE_DIR PINNED_QUICKJS_C` exports the tables.
