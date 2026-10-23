# T075: Executor instruction-address anatomy

- Status: active; requested diagnosis complete; four ranked incremental proposals await selection.
- Implementation: diagnostic collectors, source maps and evidence present; no runtime change.
- Verified at: engine source `59eb72e511cca013cf59d6736922c9c666cb883f`, 2026-10-23 UTC; 90 successful iMac profiles, seven normalized function matches, retained-count audit; final repository checks below.
- Evidence: [measurements](diagnostics/t075/measurements.md), [method and jobs](diagnostics/t075/method.md), [ranked proposals](diagnostics/t075/proposals.md), [retained data audit](diagnostics/t075/verify.py).
- Unresolved: precise hardware stall mechanism; line-zero semantic attribution; no causal savings, new wall/instruction standing or performance acceptance.
- Next action: select proposal 1's measured primitive-publication slice, obtain a fresh formal opportunity queue, then freeze/validate a finite-budget unit before runtime code. Reconcile overlapping T045 work before proposal 2.

## Result

The historical 44.21% "execution/loop" bucket is **46.4036%** on the requested
post-T069 source with the same T068 owner taxonomy. This is a new sample,
not a wall regression claim. Six executors account for **39.3127%** of all
sampled main-thread self time; the other **7.0910%** is visible helper work.
The bucket contains substantial register traffic and semantic operations.

| Inside the six executors | Main-thread self % | Executor-relative % |
| --- | ---: | ---: |
| Operand/register/Value transport and clearing | 7.9273 | 20.16 |
| Opcode fetch, PC checks, decoding and dispatch | 5.7042 | 14.51 |
| Activation/call/return preparation | 4.5870 | 11.67 |
| Indexed access and its guards | 4.0688 | 10.35 |
| Value tags, primitive clone selection and conversion | 3.9912 | 10.15 |
| Named-property/cache probes | 3.2790 | 8.34 |
| Local/upvalue routing and cell access | 2.6477 | 6.73 |
| Arithmetic/comparison and scalar operation guards | 2.6452 | 6.73 |
| Identified operand bounds compare/branches | 0.5090 | 1.29 |
| Result/completion plumbing | 0.2079 | 0.53 |
| Separate typed-loop entry/residency guards | 0.1362 | 0.35 |
| Identified inlined Rc count manipulation | 0.0366 | 0.09 |
| Unresolved semantic context | 3.5726 | 9.09 |
| **Total** | **39.3127** | **100.00** |

These are disjoint time-sampling attributions, not instruction-event counts,
removable work or causal stall prices. Visible ownership functions still own
12.1578% outside the executors. Identified dispatch is not the whole loop
bucket; the unresolved row is not silently charged to it.

Direct stack-source moves cover an overlapping **8.2201%** of main-thread
samples (wide 6.3425%). The hottest wide PCs are stack loads in
`mem::replace<Value>` → `execute::store` at Move/Dup publication
(`0x100100e8e`, 0.7670%), clear-window publication (`0x100103c46`, 0.6912%),
and a source-line-zero store (`0x100100db5`, 0.6168%). Numeric result store
`0x10010353d` is 0.4798%. The [measurement report](diagnostics/t075/measurements.md)
retains top PCs, inline source lines, all program distributions and NG comparison.

## Method and mapping confidence

Canonical fat-LTO code is sampled through copies with stripped symbols and a
new UUID; every `__TEXT` section's address/bytes/hash is unchanged. Ordinary
`sample` otherwise merges PCs by symbol, even after stripping. Ten development
programs × three repeats × canonical/full-debug/NG = 90 checksum-valid profiles;
no held-out input is resolved. Main-thread samples: canonical 90,955,
full-debug 90,792, NG 31,182. Zero merged engine self counts remain.

Full Rust debuginfo changes codegen (wide +352 bytes, VM +384), so its profiles
are supplementary. A local line-tables-only x86_64 cross-build supplies source
mapping: all six executor instruction sequences and offsets match after
normalizing relocation addresses and excluding declared jump-table data.
Registers, non-address immediates, local branches and callee identity are
preserved. NG's `JS_CallInternal` also matches its out-of-tree `-g` rebuild.
This is normalized instruction identity, not identical binary bytes or a
claim that referenced data bytes were audited. All 7,641 executor PCs map;
33,543 retained PC-count rows conserve every executor self count.

NG `JS_CallInternal` owns 48.3885% of its main-thread samples. Identified inline
transport is 6.4203%, property lookup 7.6742%, Rc duplicate/free 3.3055% and
explicit decode/dispatch/polling 2.8742%; 19.4523% remains unresolved/other,
mostly source line zero. Different inlining and denominators prevent treating
these as Rust excess-cost prices. The brief's wall/instruction ratios remain
historical context, not new results.

## Ranked next units

See [proposals](diagnostics/t075/proposals.md) for files, scope, proof
obligations, measured subpopulations and historical failures. None is approved.

1. **Destination-aware primitive register publication** — transport 7.9273%
   envelope, replace contexts 4.3949%; `compact_fn/execute.rs`, `vm_bindings.rs`,
   wide activation. Medium size/risk, high layout sensitivity. Distinct from
   T070's rejected recovery-aware ownership-transfer unit.
2. **Keep numeric indices scalar across boxed consumers** — indexed 4.0688%,
   big.js 26.687%; typed-loop compiler/provenance/recovery. Medium size,
   higher recovery risk; coordinate with T045. T070 already tried borrowed
   dense scalar reads; T030/T044 do not justify a representation/tier retry.
3. **Split shared-slot hot cache from cold variants** — `probe_hot` context
   1.7509% within named/cache 3.2790%; cache and slot-read helpers. Medium size,
   very high layout risk. T041/T049/T053 failed payoff/control gates; T072's
   incomplete screens and T073's address-dependent tails remain binding history.
4. **Prelink received-upvalue indices** — route 0.7328%, received-index
   computation 0.4236%; IR metadata and wide admission/activation. Small–medium
   size, medium semantic risk, layout sensitive. T027's local-authority rewrite
   and T069's already-promoted call preparation are different mechanisms; the
   whole-development envelope here is below one percent.

No dispatcher, error-ABI, allocator or general executor rewrite is justified
from these shares. T024/T028/T026 already demonstrate why a large symbol
or reduced dispatch count alone is insufficient. T073 requires treating
ASLR-on fresh-process tails and layout sensitivity as real candidate risks;
this study does not identify a winning address or a CPU-stall mechanism.

## Scope and verification

Owner `agent/t075-executor-anatomy/codex`, base `59eb72e5`, worktree
`t075-executor-anatomy`. Only this task, `tasks/README.md` and
`tasks/diagnostics/t075/` change. No runtime, third-party, dependency, public API,
benchmark, compiler-policy, acceptance-policy, origin push or merge change.
All iMac builds/sampling use the queue; latest job starts at 02:43:29 CST,
before the 02:45–05:00 no-start window. Sampling ends at 02:47:55; no new job
starts inside the window. Failed collectors and debug pilots remain recorded.

```sh
python3 tasks/diagnostics/t075/verify.py
./scripts/check-touched.sh --staged --explain
QJS_CHECK_SKIP_TEST262=1 ./scripts/check.sh
```

Retained-data verification passes. Fast/full final results and cleanup are
recorded in [verification](diagnostics/t075/verification.md). The curated
Test262 run is explicitly skipped for this diagnostic/doc-only change;
no runtime/parser/lexer source changes, so no focused semantic slice applies.
There is no new conformance or performance standing. No build output is committed.
