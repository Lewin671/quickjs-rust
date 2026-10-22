# T065 independent design review (dsh, deepseek-flash, 2026-10-22)

Consult-mode review of design commit `9941a257` by a different model than the author (Codex). Read-only; it ran no benchmarks.
Lead outcome and the user decision follow the review text.

### Verdict

The evidence file is unusually honest, and the destination architecture is coherent. But the design is **over-scoped relative to its own acceptance target**, **under-budgeted relative to its scope**, and it **repeats two mechanisms this repository has already measured as losses**: a shared-window frame driver (T021: 1.2678) and instruction-count reduction as the primary payoff (T021 virtual stack: −21.6% ops, 1.0149 overall). The central decision is not "is this architecture buildable?" It is: **is the goal ~2.46× NG (the proposed 0.80×M) or closing the gap?** If 2.46× is acceptable, a full core rewrite is not the cheapest route. If parity is the goal, the design is under-ambitious and 24–40 engineer-weeks is too small.

I read the documents and sources only; I ran no benchmarks.

---

### 1. Does the gap anatomy support the chosen architecture?

**Partly. It supports the mechanism family, not the specific package.**

What the evidence actually establishes:

- Value ownership/transport is real: 29–139M `Value` clones per program, 42–91% heap-owning except big.js at 15%; 8–15% visible glue, with more hidden by inlining ([part 2 counters](tasks/archive/T056-unit0-part2-evidence.md)).
- Named property authority is real: 6–28% named-owner shares, 4.0/5.7 name comparisons per attempt in moment/diff, 46–93% cache rejects ([anatomy](tasks/archive/T065-gap-anatomy.md), [counters](tasks/archive/T056-unit0-part2-evidence.md)).
- Call/env plumbing is real but smaller than the table's "0–27%" suggests: T056's independent nested-VM work is only 3.5% mean (max 10.3%); env construction is the larger part, and T021's frame-stack experiment says removing activations can cost more than it saves.
- The 16-byte `Value` and 112-byte `Op` are genuine per-op taxes: `size_of::<Value>() == 16` and `size_of::<Op>() <= 112` are asserted in-tree. T021's own conclusion is explicit: the residual per-op cost "is the `Value` representation — 16 bytes, `needs_drop`, three `Move`s per operation" ([T021 log](tasks/archive/T021-single-vm-frame-stack-log.md)).

What the evidence does **not** establish:

1. **Per-mechanism attribution.** The anatomy says this plainly; the "visible gap-equivalent reach" numbers are overlapping conditional envelopes, not additive causes. That is fine as far as it goes, but it means the architecture is a hypothesis, not an evidence conclusion.
2. **That dispatch/register bytecode is a major lever.** T024's dispatch-only experiment was 0.9906 external / 0.9989 sentinel. T021's virtual stack cut operations 21.6% and still lost overall (1.0149), with a 6.5% win only on recursion. A 15% instruction cut may buy 3–6% wall, not 15%.
3. **That the 23% wall/instruction residual is uniform.** It is not. Using T040/T057 wall medians and the T065 instruction medians, the implied wall-per-instruction ratio ranges from ~1.10 to ~1.51:

| Program | Wall / NG | Instructions / NG | Implied wall per instruction |
|---|---:|---:|---:|
| js-yaml | 4.34 | 3.47 | 1.25 |
| immutable | 4.01 | 2.65 | **1.51** |
| moment | 3.65 | 2.58 | **1.42** |
| diff | 3.62 | 2.91 | 1.24 |
| meriyah | 3.34 | 2.61 | 1.28 |
| lodash | 2.75 | 2.16 | 1.28 |
| big.js | 2.44 | 1.89 | 1.29 |
| matter-js | 2.22 | 1.75 | 1.27 |
| graphlib | 2.81 | 2.17 | 1.30 |
| papaparse | ~2.27 implied | 2.07 | ~1.10 |

The nine-program CPI geomean is ~1.32; the aggregate 1.29 hides 1.10–1.51. The high end is object/allocation-heavy (immutable, moment), the low end is string/parse-heavy (papaparse, diff). This matters: **the 23% residual is a different mechanism on different programs**, and the design's "measure instructions, expect wall" checkpoint cannot see it.

4. **Tier share.** The anatomy does not say what fraction of each real program runs in the general VM vs compact/wide/typed. That is the single most important missing number. The current engine is at parity on SunSpider/JetStream/Kraken but 3× on npm libraries; that strongly suggests the libraries are hitting a path the synthetic suites avoid. If big.js runs 99% in the compact tier and the synthetic suites run in typed kernels, then "replace the interpreter core" means different things for different programs.

5. **Startup/parse/compile.** Whole-process instruction counts include parse/compile/startup. T064's closure notes startup instructions +7.1% and unresolved unpack allocations. The design has no startup lane and no compile-time budget. For the 86-case portfolio, startup can dominate a short case; a 0.95 portfolio gate can be lost there while the interpreter improves.

6. **Allocator and code layout.** Allocator self is 10–25% in seven development programs. T046 already rejected PGO/native CPU tuning at this revision, and T064 showed object-model changes moving typed code layout and failing 46/86 instruction pre-gates. The design's answer is an arena, but it does not price a global allocator swap, nor does it measure cache/branch behavior. A `mimalloc`/`snmalloc` experiment is one line and a week; it should be a control before attributing allocator gains to GC.

**The NG counterexample is the strongest evidence against the package.** QuickJS-NG is a 16-byte `JSValue`, a **stack** VM with short byte-optimized opcodes, **no per-site inline caches**, a refcount + backup cycle collector, and computed goto ([quickjs.h](third_party/quickjs-ng/quickjs.h), [quickjs.c](third_party/quickjs-ng/quickjs.c)). It is 3× faster on real libraries. So the gap is not "Rust lacks V8-style shapes and ICs." It is per-operation cost: value width/drop glue, call/argument construction, property name comparison, and allocation. The design's atoms/shapes are a good way to fix the property part, and the 8-byte word is the right way to fix the value part. But **ICs are an optional optimization NG doesn't need**, and **register bytecode is a means to compactness, not a proven payoff**. The design should demote both from "core pillars" to "supporting changes with measured gates."

**Which mechanisms would actually close most of the gap?**

In priority order, on this evidence:

1. **8-byte value with cheap copy semantics** (no 16-byte enum, no `needs_drop` glue on every move). This is the one T021 explicitly identifies.
2. **Property authority by atom/shape/slot**, eliminating string hashing/comparison and descriptor cloning. The existing `NamedPropertyCache` is already 4-way polymorphic with prototype entries; the delta is shape identity and slot storage, not "add ICs."
3. **Call/argument/env plumbing**: remove `Vec<Value>` per call and the nested `Vm`/`CallEnv` for ordinary calls. T056's M2.1/M2.3 reach is ~10% of sampled development time; that is real, but T021 says the *window sharing* part is where it goes wrong.
4. **Allocator/arena**: 10–25% self, and a fast size-class allocator plus a value-width reduction is the cheapest locality win.
5. **Compact instruction encoding**: useful for I-cache and fetch, but not sufficient alone.
6. **Strings/RegExp**: necessary for js-yaml/papaparse, but this is instruction-count work, not the high-CPI residual. Deferring it to M2 is defensible only if the worst wall cases are allowed to stay bad until then.

**What is big and missing:** a per-program CPI/cache/branch measurement (`perf stat` or cachegrind on Linux), a tier-share inventory, a startup/compile lane, an allocator control, and a pre-M0 ablation ladder. The design jumps from "attribution is unidentified" to a full package without a cheap falsification step.

---

### 2. Is the architecture right for safe-ish Rust with audited unsafe?

**It is a coherent destination, but not the lowest-risk one for this codebase.** The current runtime has **no `unsafe` blocks** (the only grep hit is a test name), 437 `RefCell` borrow sites, and a 16-byte enum `Value`. A packed word + tracing GC + windowed frames would introduce a large audited-unsafe surface and a pervasive *safe-code* root discipline. That is a bigger change than the unsafe count suggests.

The architecture is right about: nonmoving arena first (no write barrier), atoms as `u32` per runtime group, shape/slot storage, slot-indexed bindings with shared cells, compact bytecode, one call ABI, explicit completion/throw, no conservative stack scan. Those are the correct direction.

The parts I would change:

### 2.1 The tracing GC is bundled with the value word; separate them

The evidence supports an 8-byte word. It does **not** support a full precise tracing GC over an 8-byte refcounted alternative. NG proves the hybrid: refcount frees the common case, and `JS_RunGC` does decref/scan/free-cycles for cycles ([quickjs.c](third_party/quickjs-ng/quickjs.c)). That design needs **no precise root maps for normal values**, which is the single biggest soundness advantage in Rust. The design dismisses it in one sentence ("an 8-byte word with RC keeps most ownership work") without measuring a *custom header* refcount versus Rust `Rc<enum>`. A `#[repr(transparent)] struct Value(u64)` with an inlined `Clone`/`Drop` (one tag branch + count) is not the same cost as today's 16-byte enum with `Rc` and drop glue. This deserves a 2–3 week A/B before committing to tracing.

If tracing is retained, the safety story needs more than "Miri on the safe path + sanitizers." Miri cannot validate a custom GC; ASan cannot catch a use-after-free when the arena cell is reused from a free list. You need debug-only **epoch/tag checks on every handle dereference**, a poisoning collector, and a root-omission stress mode. The current plan has the right tests but not the mechanism that makes them catch the bug.

### 2.2 The shared-window frame driver repeats T021

T021 built exactly the mechanism the design proposes — one register buffer for the whole chain, each activation a window at `base`, call/return as a window switch — and measured **1.2678 (27% slower)**, with only 2,001 activations instead of 254,001. The conclusion was explicit: "every register access now costs a `base +` offset... the nested design wins here precisely because each activation's registers start at zero and the compiler can keep the base in a register" ([T021 log](tasks/archive/T021-single-vm-frame-stack-log.md)). The design's risk list mentions T021 but only says "reusable storage must remove entry work and use fieldwise initialization" — that is not an answer to the base-offset cost. The T056 frame design also says "window bases are store indices", which is the same trap ([frame-activation.md](docs/design/frame-activation.md)).

A correct variant is **zero-based per-activation slot blocks in chunked storage**: the frame directory may move, but each frame's slots start at index 0, so the hot loop keeps a base pointer in a register and pays no per-access offset. That preserves the "no nested Vm" win without repeating T021. This should be a required design decision and a small A/B before M0.

### 2.3 Packed pointers and `Copy` words are the rooting hazard

The design says "a word alone is never a host root", "no reference survives an allocating or re-entrant operation", and "no Rust object/property borrow may span allocation". Those are correct invariants, but Rust will not enforce them on a `Copy` `u64`. A `let v = word; allocate(); use(v)` is safe code and compiles. There are ~3,400 direct `Value::` matches and 437 borrow sites; the "value access facade" is used in only ~25 places, so it is not insulating the representation ([value/access.rs](crates/qjs-runtime/src/value/access.rs)). The audit surface is the whole runtime, not `core_v3/{word,heap}`.

At minimum:

- Make transient words a distinct, non-`Copy`/private type; anything crossing a safepoint uses a `Rooted<'scope>` token or an explicit root scope. Do not let safe code hold a raw `ValueWord` across a call.
- Make `ArgRange`/window accessors return generation-checked or rooted handles, not raw words. A native that keeps an index across a callback is holding a dangling window if the store grows.
- Use **indexed handles** (page, slot, generation) as the default, or at least as the debug backend, instead of integer-encoded pointers. It avoids provenance and ABA issues and costs one load. The design keeps indexed handles only as a fallback if M0 fails.
- Define the heap borrow API before M0. "No borrow spans allocation" is a rule, not an API; the code needs a `with_object`/indexed access pattern that cannot compile the violation.

### 2.4 Smaller but real hazards

- **OOM:** "OOM leaves roots and frame state valid" is not implementable with default `Vec`/`Box` allocation in Rust — allocation failure aborts. Either specify fallible allocation everywhere (a large project) or drop the claim.
- **Finalizers/weak refs:** the design sequences them correctly in M3, but "native resource release cannot execute JS or resurrect swept cells" needs a concrete destruction order and a no-allocate-during-sweep rule.
- **Agents:** the current model is one engine per OS thread, values never cross threads ([agent.rs](crates/qjs-runtime/src/agent.rs)); a per-runtime heap is fine. State that explicitly so a future shared-heap attempt doesn't silently break the GC.
- **Packed pointers on 48-bit/57-bit Linux:** the "check every allocation's full address" is right, but failing allocation when an address is high is a poor production behavior. Indexed handles avoid this entirely.

---

### 3. Is the migration realistic? Is 24–40 engineer-weeks credible?

**The shape is right; the granularity and the estimate are not.**

The opt-in backend inside `qjs-runtime`, vertical builtin families, no live cross-heap adapter, and keeping the default engine as the oracle are all sound. Rejecting the universal graph adapter is correct: it would preserve the old cost and complicate identity/cycles. Keeping the old public API until cutover is correct.

But "no cross-heap adapter" plus "no old Vm/CallEnv/compact executor may execute in the v3 process" makes each program a big-bang port. M0 must have all big.js builtins; M2 must have **all** standard builtins, eval/with, classes/private/derived, modules, and strings/RegExp before the ten programs run. That is not a strangler; it is a parallel engine with a compatibility wall. It increases the chance that 15–24 weeks pass before a real program is validated.

Some concrete numbers:

- The runtime is 189k lines **including 49k test lines**; production is ~133k. The design's "roughly 189k runtime lines" overstates the port surface, but 133k is still large: bytecode 73k, regexp 8.4k, function 8.1k, value 6.5k, typed_array 6.2k, array 4.4k, string 3.6k, object 3.5k, module 3.5k.
- M2's 6–10 weeks for all builtins + eval/with + private/class/derived + modules + strings/RegExp is the most implausible line. Even mechanical porting at 3–5k lines/week is 20–40 weeks for the builtin surface, before semantic debugging. 12–20 weeks is more defensible.
- M4's 3–5 weeks to port typed numeric/array kernels and scalar replacement is not credible after T064. The specialized executors (`compact_fn/wide`, `typed_loop`, `vm_numeric_mutation_loop`) are ~15k lines and the repo spent months on kernel isolation, failing 46/86 instruction pre-gates. Porting them to a new frame/root ABI in 3–5 weeks is the single most under-estimated line in the table.
- M0's 4–6 weeks with "20 days to first working program" is optimistic by roughly 2×. A precise GC + root discipline alone is 2–3 weeks; a compact register lowerer/verifier is 2–3 weeks; the big.js builtin closure is 2–3 weeks; the frame driver/native ABI is 2–3 weeks. Ten to twelve weeks is a realistic range for the stated M0 scope, or six weeks for a *cut* scope.

A defensible one-owner range is **45–70 engineer-weeks**, not 24–40, with the 30–50 contingency still low. With 2–3 owners after the core API stabilizes, M2/M3 builtin families can parallelize, and 30–45 calendar weeks is plausible. The document should state the staffing assumption; "more agents do not linearly shorten shared-value or GC work" is true but does not justify a single-owner estimate for parallelizable builtin ports.

### Is there a cheaper path to most of the gain?

**Yes, and the design does not price it.** The proposed acceptance is 0.80×M, i.e. ~2.46× NG. T056's own reach inventory sums to roughly 20% of sampled development time across M2.1–M2.4, and the value/allocator work adds more. A staged incremental path on the existing engine could plausibly hit 0.80–0.85×M in ~15–20 weeks:

1. 8-byte value backend (custom header refcount first, tracing later if justified) behind the existing object model — 4–6 weeks.
2. Fast allocator / size-class arena for object payloads — 1–2 weeks.
3. Compact 16/32-bit general bytecode lowering (keep compact/wide tiers) — 3–4 weeks.
4. Typed per-activation entry, no `Vec`/`CallEnv`/nested Vm, **without** a shared window — 3–4 weeks.
5. Atom/shape/slot property authority on the existing caches — 4–6 weeks.

Each stage is independently gated, preserves the tiers and conformance, and reuses code if the full rewrite later proceeds. The design rejects "keep Rc and replace dispatch first" — correctly — but that is not the same as this path, which changes value width, allocation, calls, and property authority. The design has not shown that this path is capped below 0.80. Until it does, the rewrite is a **risk-appetite choice**, not an evidence conclusion.

---

### 4. Is M0 the right first risk-retiring slice?

**The program choice is defensible; the mechanism set and gates are wrong.**

Big.js is a real program with a small builtin closure, and using the complete program rather than extracted arithmetic is the right principle. But:

- **Big.js runs almost entirely on the compact/accelerated path, not the general VM.** The counters show 56 nested VMs, 0.03M general calls, 0.66M accelerated named accesses vs 0.02M VM named, 85.1% loop, 15.5% glue, 1.5% named, 0.1% call. So M0 is primarily a test of the new **value word/GC and compact per-op cost**, not of the general frame driver, property ICs, or call paths. The design says it exercises "constructor/prototype/array/string/numeric behavior" — true — but the profile cannot validate the 0–23% property and 0–27% call hypotheses. Graphlib at M1 is the first call test, 11–14 weeks later.
- **The NG instruction gate is slack.** Big.js is already 1.8877× NG. The `≤2.0 NG` gate is satisfied by the baseline; the only binding test is `≤0.85×M`, a 15% instruction cut. That is a weak leading indicator for a 0.80 wall target.
- **The instruction gate is not predictive of wall.** T021's virtual stack is the direct counterexample: 21.6% fewer ops, 1.0149 overall. A 0.85 instruction ratio could easily be 0.95 wall. If M0 passes on instructions but the wall is 0.95, the team will have spent six weeks and learned the wrong thing.
- **The scope is the whole architecture at once.** Packed words + arena GC + shapes + register bytecode + single driver + native ABI + all big.js builtins means a failure cannot be attributed. That is the opposite of risk retirement.

**A better M0:**

- **M0a (2–3 weeks):** safe reference word + arena GC + per-activation zero-based frame storage + compact lowerer on a small real subset; no shapes/ICs, no new native ABI. Gate: GC stress/root tests, big.js arithmetic instruction and **wall** improvement, RSS.
- **M0b (3–4 weeks):** packed word swap + all big.js builtins + compact executor on the new value/object model. Gate: complete program, zero legacy, frozen Test262 slice, wall/instruction/RSS.
- Keep the compact/wide executor during M0; port it to the new value/heap rather than requiring the new general driver to beat it immediately. This isolates the value/GC risk (the highest) from the frame-driver risk (T021's proven trap).
- Add a **call/property microbenchmark** (or a pre-declared graphlib subset) to M0 if the frame driver is in scope; otherwise move the frame driver to M1 and say so.
- Add a week-3 go/no-go: if the collector/word/basic loop isn't working, stop before building builtins.
- Add a **wall** gate: e.g. M0 median wall ≤0.90×M, M1 ≤0.88×M, with K=6 layout confirmation.
- Tighten M0 RSS from 1.25 to 1.10 and require live-bytes return to baseline.

If the lead insists on the complete big.js M0, the cap should be 10–12 weeks, not 6, with the same sub-checkpoints. The 6-week cap with the current scope is a schedule failure waiting to happen, and it will stop a viable route for the wrong reason.

---

### 5. Are the acceptance thresholds sensible?

Mixed. The structure is good — development, every lane, portfolio, per-case, held-out, RSS, Test262 — but several values are either too tight for the migration plan or too weak for the stated goal.

| Gate | Assessment | Recommendation |
|---|---|---|
| Development wall geomean ≤0.80×M | Sensible as a phase-1 target, but it leaves ~2.46× NG. It does not close the stated gap. | Decide explicitly whether 2.46× NG is acceptable. If not, set ≤0.60–0.65×M and extend the budget; if yes, say this is phase 1 and size the architecture to it. Add a **worst development case ≤0.90×M** so js-yaml/immutable cannot hide inside the geomean. |
| Every lane/suite ≤1.02×M | Too tight if compact/wide/typed executors are retired. T064 failed 46/86 instruction pre-gates from layout coupling; a full backend switch will move external lanes. | Keep the tiers through M5, or set 1.05 for phase 1 and 1.02 as a later target. Do not require both "one generic core" and "no 2% regression on the suites the specialized tiers serve." |
| 86-case portfolio ≤0.95×M | High but defensible if the broad lane improves. Currently broad is 0.990×NG, so 5% is a real ask. | Consider 0.97×M with per-lane controls; report broad separately. |
| Every case ≤1.10×M | Reasonable, but with 95 cases and layout noise, a few outliers are likely. | Keep, with a pre-declared layout/precision exception process, not a post-hoc threshold change. |
| Held-out 9-case geomean ≤1.00×M | Small sample; a 1.00 geomean can hide a 1.20 case. | Use ≤1.03 geomean and ≤1.10 per case, or enlarge the held-out set. |
| RSS ≤1.05×M for all 95 | Too tight for a nonmoving arena with page retention and size-class waste, especially on memory-light cases. | 1.05 geomean, 1.10 per case; add post-GC live/retained bytes and a page-release test. |
| Test262 zero configured gaps | Already true: the ledger's latest complete scans are 42,672 configured, Rust 42,672 pass, 0 actionable gaps ([burndown](docs/conformance/burndown.jsonl)). This is a **preservation** obligation, not a future achievement. | Reframe as "lose none of 42,672 at every milestone." Freeze the supported slice per milestone; run a complete exact scan at M1/M2, not only M3/M5. A raw "remaining gaps" count at M2 is meaningless unless the claimed slice is frozen and 100% passes. |
| Instruction gates | Diagnostic, but used as stop gates. `≤2.0 NG` is slack for big.js (1.89) and graphlib (2.17). | Use `≤0.85×M` as the binding gate and report NG for context; or tighten NG to 1.70/1.85. Add wall/CPI. |
| Formal protocol | The K=6 layout panel, sealed evidence, formal upper bound, and two-candidate budget are good. | Keep. Add a **GC pause** gate (p99/max), a **startup/parse** lane, and an **allocation rate/live bytes** gate. |

The proposal's statement that 0.80 "does not promise parity" is honest, but the user's motivation was a 3.07× gap. The lead should sign off on the target explicitly before the architecture is frozen. A 24–40 week rewrite that lands at 2.46× NG is a 20% improvement; that may be the right business call, but it should be stated as such, not implied by "rewrite the core."

---

### Concrete, prioritized recommendations

### P0 — before approving the design or starting M0

1. **Reconcile the target with the architecture.** Decide 2.46× NG vs parity. If 2.46× is enough, price the 15–20 week incremental path first. If parity is the goal, the 0.80 target is too weak and the 24–40 week budget is too small.
2. **Run a 4–6 week pre-M0 ablation on 3 programs (big.js, immutable, js-yaml).** Compare: (a) 8-byte custom-refcount word vs non-owning word, (b) global allocator, (c) compact general bytecode, (d) typed per-activation entry without a shared window. This is cheaper than M0, produces reusable code, and directly tests the package.
3. **Replace the shared-window frame driver with zero-based per-activation slot blocks** in chunked storage, and run a T021-style A/B before committing. If the shared window is retained, the design must explain why base+offset will not repeat 1.2678.
4. **Keep compact/wide/typed executors through M5.** Port them to the new value/object/frame API; do not require the general driver to absorb them in M0/M4. This removes the biggest source of the 1.02 lane risk.
5. **Split M0 and add wall/CPI gates.** M0a: word/GC/frame on a small real subset. M0b: complete big.js. Add a call/property microbench. Re-estimate 45–70 engineer-weeks for one owner; state staffing.

### P1 — change the design

6. **Separate the value word from the collector.** Make custom refcount + cycle collector the primary candidate, tracing the fallback, unless the A/B proves otherwise. This matches NG and removes the precise-root hazard from most of the runtime.
7. **Make roots type-enforced.** A non-`Copy`/rooted handle for anything crossing a safepoint; generation-checked window handles for natives; indexed handles or debug epochs for the packed backend. Add a poisoning/epoch debug GC and a fuzz target; Miri/ASan are not sufficient for a custom GC.
8. **Freeze Test262 slices per milestone and scan the complete configured suite at M1/M2.** Treat 42,672 passes as a preservation budget; no milestone may claim "supported slices" without a pre-declared list.
9. **Price a counted, temporary cold compatibility boundary for M2** (or explicitly justify the big-bang builtin port). Waiting 15–24 weeks to run a real program is a large avoidable risk.
10. **Add atom-ID shape hit/reject gates** (e.g. ≥90% own-slot hits on immutable/graphlib, bounded shape/atom retention under churn). The existing 46–93% reject rate is the number the new design must move; otherwise the IC work is speculative.
11. **Define the heap borrow/access API and drop the OOM-validity claim** unless fallible allocation is in scope.

### P2 — acceptance/tooling

12. **Adjust gates:** worst development case ≤0.90×M; lane 1.05 phase-1 (or keep tiers); portfolio 0.97; held-out 1.03 geomean; RSS 1.05 geomean / 1.10 per case; instruction gates use M, not the slack NG bound.
13. **Add gates for GC pause (p99/max), startup/parse wall, allocation rate/live bytes, and cache/branch behavior.** A 100 ms STW pause on a 4 s program is 2.5% wall — enough to fail 0.80.
14. **Measure the 23% residual directly.** Run the ten bundles under `perf stat`/cachegrind on Linux (instructions, cycles, cache-misses, branch-misses, page faults) to distinguish allocator, cache, code layout, and dispatch. The current evidence has no hardware counters.
15. **Correct the small factual overstatements:** the port surface is ~133k production lines, not 189k; `ValueKind` is used in ~25 places, not as a representation facade; NG has no ICs and uses a stack VM + refcount/cycle GC, which is the alternative the design must beat.

---

### Where the design is plainly wrong or over-scoped

- **The one-frame-driver pillar is the clearest technical error.** T021 built the shared window and measured 27% slower. The design's risk paragraph acknowledges T021 but does not resolve the base-offset mechanism.
- **Bundling a precise tracing GC with the 8-byte word is not supported by the evidence.** The word is supported; the collector is a separate, large, soundness-heavy bet. NG gets 3× with refcount + cycle collection.
- **The 24–40 week estimate is not credible for the stated scope.** M0 at 6 weeks and M4 at 3–5 weeks are the worst offenders; 45–70 one-owner is more defensible.
- **The 0.80×M target does not close the 3.07× gap.** If that is the goal, the architecture is over-scoped. If parity is the goal, the target is under-ambitious and the budget is too small. The design cannot be both a parity rewrite and a 20% incremental gain.
- **M0 does not retire the riskiest assumptions.** Big.js is mostly compact-tier execution; it tests the value word and per-op cost, not the property/call/frame hypotheses. The NG gate is already satisfied at 1.89.
- **The conformance gate is already satisfied at baseline.** Treating "Test262 zero gap" as a future payoff misreads the ledger: the engine is at 42,672/42,672 configured with zero actionable gaps. The real risk is losing passes, and the plan defers that discovery to M3/M5.
- **The "no cross-heap adapter" purity plus "all builtins before real programs" turns a strangler into a big-bang port.** It is a defensible purity choice, but it is a major cost driver and should be priced, not presented as free.

## Lead outcome (2026-10-22)

The lead accepts the review's main points: the 0.80×M target leaves about 2.46× NG; the shared-window frame driver repeats T021; the tracing collector is bundled with the 8-byte word without evidence; the estimate is low (45–70 one-owner weeks is more defensible); M0 on big.js cannot test the call/property hypotheses and its NG gate is already met; Test262 is a preservation obligation.
The user chose (2026-10-22) to run a pre-M0 mechanism ablation on real programs before choosing between the rewrite and an incremental path. The architecture, migration and acceptance proposal stay unapproved until its results.
