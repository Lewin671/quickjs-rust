# T077 S3: landable execution rebuild proposal

- Status: approved by the lead 2026-10-29 with amendments in [T077](../T077-core-rebuild.md) (S3a may land before S1/S2; S3e prices growing the wide tier).
- Implementation: S3a operation entries and native slice transport present on its owner branch; S3b–e remain planned. See [S3a results](../diagnostics/T077-s3a-results.md).
- Verified at: S3a review repair `03c8680c`, 2026-10-30; full check/257 NG fixtures, depth parity against base and twenty deterministic development runs pass; fresh complete exact scan pending.
- Evidence: [counts/reproduction](../diagnostics/T077-s3-counts.md), [Op map](T077-s3-op-map.md), [construction inventory](T077-s3-activation-inventory.md), [native/executor inventory](T077-s3-native-executors.md).
- Unresolved: fresh S3a complete exact scan/burndown, lead formal aggregate acceptance/integration and T074 contracts before S3b.
- Next action: finish review-repair exact verification, then lead formal S3a acceptance and T074 review.

## Recommendation and authority at every landing

Land five replacements, with native entry/call transport and Context first.
Do not build universal Code and retain a second universal interpreter during a
long cohort migration. S3e replaces the whole instruction authority and deletes
the leaf/compact/wide/numeric/virtual executable paths in the **same landing**.
Existing not-yet-replaced accelerators remain named and bounded until then;
there is no newly introduced fallback backend or code-selected language cohort.
Each sub-stage has production behavior and deletions, not dead infrastructure.

All stages inherit [T077 acceptance](../T077-core-rebuild.md#stage-acceptance-overrides-ordinary-unit-gates-for-t077-only): focused tests,
full check, every NG fixture, complete configured exact comparison with zero
gaps/lost passes/new exclusions, complete-scan burndown, and debug stress plus
feature configurations. Freeze base, APIs, tests, deletions and aggregate rule
before code. No stage lands merely because a later stage might fix correctness.
Formal wall evidence uses clean non-counter binaries and sealed held-out
acceptance only; these counts select scope, never prove speedup.

S3a/b are proposed payoff landings (all four aggregates improve). S3c may be
neutral only with named payer S3d, which must improve all four and development
against the accepted S3b endpoint. S3e may be neutral only with S4 as already
specified by T077; retain S4 versus accepted S2 and all original milestone-pair
comparisons. Lead must freeze these extra endpoints, not silently reinterpret
S1–S7 gates. If a payer fails, stop and repair/revert its enabling pair; no
source-level old-path fallback. The fresh lead baseline remains pending here.

| Stage | Scope and deletions | Counted reach / expected payoff, not a forecast | Size; parallelism |
| --- | --- | --- | --- |
| S3a native entries and slice transport | Bind every native operation at installation; native call/construct and host/callback slice entry. Delete native_family cache, 28-family Option probe chain and ordinary native argument Vec copies. | 10,986,795 family/core calls; 78,693,537 declined family probes, including 28 declines for each of 2,810,361 core calls. Native slice bypass also reaches some of 7,438,472 pop_arguments constructions; do not claim all are native. | 1–2 engineer-weeks; entry/installation owner, array and string-family API tests in parallel; no S1-owned paths. |
| S3b typed Context + T074 binding authority | Replace CallEnv everywhere, scope leases/BindingPlan/context fields; delete empty-env factories, slot-name snapshots/apply-back and caller-env equality proof. | 24,987,337 CallEnv constructions/clones across ten programs; fresh frame factory alone 20,665,887. Early payoff is service/context deletion on current IR, not bytecode compression. | 3–5 weeks; one shared Context/binding owner; family capability ports parallel after API freeze. T074 approval required. |
| S3c owning activations and suspension reuse | One fieldwise activation/block owner and bounded pool; generator/async/TLA suspend/reattach same block. Delete GeneratorSnapshot reconstruction and by-value FrameState ownership protocol. | 2,137,517 frame/VM constructions and operand-vector requests establish surface; suspend-specific savings are **not** priced by the development corpus. Enabling stage; S3d pays. | 2–3 weeks; one activation/storage owner, suspension/jobs owner after ownership ABI freeze. Serialized with S3b. |
| S3d explicit JS activation stack | General call/construct, eval and effectful JS hooks enter one driver; direct writes into callee storage, cold semantic continuations, checked native boundaries. Delete nested JS Vm creation, dormant frame_stack machinery and ordinary JS argument Vec transport. | 2,137,517 old VM constructions, 7,438,472 pop-argument vectors, 448,836 tier-fallback vectors, 5,259 slice fallbacks. Retain semantic spread/bound/rest arrays and expose their counts separately. | 3–5 weeks; lead driver/call owner, effect-continuation family owners after request ABI freeze. Depends S3c and settled S2 hooks. |
| S3e universal register Code and executor retirement | All 120 Op variants lower universally; verifier/single PC loop, typed/mutation exit maps. Delete executable Op, stack dispatch, numeric/this leaves, compact/numeric-plan/wide drivers, caller admissions, virtual streams and handback. | 831,246,923 compact/wide register ops versus 289,414,045 general ops; wide inline entries 18,305,031. Deletion has large throughput risk; S4 is payer, no compression-only gain promised. | 4–6 weeks; one builder/verifier owner, lead dispatcher/semantic integration, loop-map owner. S3d first; no family-by-family backend landing. |

Planning range **13–21 engineer-weeks**, plus 1–2 weeks for T074/API review and
unpriced conformance repairs; testing/review is included in each stage range.
This is a sizing estimate for one primary owner with bounded family assistance,
not a measured calendar promise. Shared ABI changes serialize. S3e is the largest
remaining landing, limited to a single code-authority switch rather than the
whole 12–20 week S3 branch. If its frozen inventory cannot fit the six-week upper
estimate, return to lead **before coding** to re-scope the representation cut;
do not create an indefinitely unlandable dual backend. Checkpoints below are
review checkpoints, never permission to land incomplete language support.

## Exact cuts, preconditions and reusable work

**S3a** owns `native.rs`, `native/*`, installation entry binding and native call
routing in `function/call.rs`/`bytecode/vm_call.rs`. Bind an operation-specific
entry, exhaustively covering NativeFunction including internal reactions/agents;
TDZ marker stays an error. Keep existing semantic family implementation boundaries.
Its temporary entry service parameter is the *existing* CallEnv, the only context
authority at that landing. `CallInfo` and borrowed arguments are already final in
shape; S3b mechanically changes that parameter to Context and ports capabilities.
This small explicit signature migration costs less than forcing the full builtin
Context port into S3a. It is not a Context adapter or a second env authority.
Lead review correction: CallInfo holds borrowed callee/receiver plus construct
mode; actual new.target/realm are lazy service queries. Generic VM argument-owner
borrowing also covers JS fallback calls to avoid a new native-kind admission load.
S3a adds no depth/stack limit; runtime-wide resource policy is S3d-owned.
Borrow immutable argument slices from an owned/detached operand/register block;
callbacks cannot recycle it. Test retained args, callbacks, throw and cross-realm
construct/new.target. Probe removal is measurable even if wall payoff fails.

**S3b** owns `function/{env,call*,arguments,upvalue*}.rs`, new context/binding
modules, `bytecode/vm_*` capability uses and all builtin/module/context consumers.
Read the [T074 proposal](T077-s3-eval-scope.md) before implementation; all thirteen
reproducers and T071 constraints are acceptance, not optional follow-up. Static
locals/captures keep positional routes; dynamic scope tables own cells only.
CallEnv cannot survive as a renamed name/value snapshot hidden inside Context.
Existing stack/compact/wide/loop executors consume typed services directly; no
bridge reconstructs CallEnv. Ordinary frames still execute current IR, so no
compiler/opcode rewrite is needed. Runtime constructors retain correct owning
realm/private/referrer/this metadata and callback re-entry restoration.

**S3c** owns `bytecode/{vm,vm_frame_init,frame_program,vm_generator,frame_stack}.rs`
and generator/async/module suspension owners. Frame directory entries reference
stable boxed activation/block owners; current activation is detached into driver
locals, not copied as a several-hundred-byte aggregate at switches. Initially
storage is existing locals/operand slices inside that block. Keep cell identities,
TDZ maps, handlers/disposal, pending completion and resume mode together; never
pool escaped cells. Root and resumed frames use the same builder. This ownership
API survives Code; replacing separate operand/local vectors by RegisterBlock in
S3e is a declared one-time storage conversion, not a second semantic port.

**S3d** owns driver/request/continuation definitions, `function/call*`, VM calls,
coercion/property/iterator/class/eval/disposal continuations and callback boundaries.
A pure JS getter, Proxy trap or ToPrimitive hook must request a callee frame and
resume a semantic step; no synchronous helper secretly builds a nested Vm.
Synchronous native callbacks may nest Rust NativeBoundary records under a checked
native-depth limit. JS depth is separately checked. Current compact/wide stacks
are unreplaced specialized executors, not a new general backend; their fallback
and re-entry now use this one driver. Call argument moves target callee slots;
rest/destructuring/default prologue and actual-argc observation remain correct.
Full hook conversion is part of this stage's estimate, not postponed behind an
ordinary-call-only admission predicate. Test deep JS recursion including hooks,
error/finally override, derived constructors, iterator close and native re-entry.

**S3e** owns `ir*`, compiler/lowerer, new Code/verifier, dispatch and removal paths
in the inventories. Reuse existing semantic helpers/Context/continuations and
zero-based activation API; do not port builtin algorithms again. Universal CFG
lowering covers try/finally, eval, classes, modules and suspension as first-class
register code. Compile existing typed/mutation metadata from ephemeral IR, remap
all registers/recovery PCs, release IR and let exits resume Code exactly once.
No `bytecode_op` callback can read retained old Op. Delete virtual executable
streams immediately; allocate ordinary objects/functions until S4 general escape
analysis recovers proven cases. Before switch review complete opcode/handler map;
before landing audit removal of plans/facts/TLS pools/cached declines/counters.
S4 retains its scalar/escape scope and deletes remaining loop execution owners.

## S1/S2 and owner scheduling

Inventory/design/counting may run beside S1 now. No S3 runtime edit before S1
lands may touch `property/`, `value/object*`, `value/layout/` or
`bytecode/linked_code.rs`. S3a's installation binding and family tests can be
prepared on disjoint paths; coordinate `function/value.rs` with S2's function
NamedStore owner. S3b capability signatures and tests can be drafted beside S1,
but its broad implementation/landing waits for accepted S2 and the T074 review.
This retains T077's S2-before-S3 integration order; early S3a landing before S2
would be a separate lead amendment, not assumed authorization.

Consumers require S1's explicit domain/key/layout constructor APIs and S2's
single named-property site/validity/exotic-hook interface. S3 never introduces a
property bypass to avoid waiting. S3c–e implement after these APIs settle; only
schema/test/inventory work is parallel beforehand. For S3b assign array capability
uses `native/arrays.rs` + `array/`, strings/regexp to their families, and other
families in bounded groups. Lead alone owns Context, FunctionData shared fields,
compiler/IR, driver, global docs and integration. No overlapping branch edits.

## T021 cost and remaining disagreement

S3a/b retain current local addressing; no shared window is introduced. S3c binds
zero-based local/operand slices once per execution segment. S3d switches detached
block owners and rebinds slices only at call/return/re-entry. S3e hot reads are
`regs[r]`, never `store[base+r]` or per-access FrameId resolution; keep PC/code/base
as loop locals, initialize headers fieldwise. At c/d/e inspect release disassembly
and non-counter development comparisons for repeated directory/base loads and
action spills. Fewer frames/probes cannot by themselves pass a wall gate: T021
removed 252,000 activations and lost 1.2678×. No unsafe indexing is prerequisite.

No disagreement with the approved final architecture. Two sequencing/estimate
amendments need lead approval: bind native entries before the whole Context port,
and make owning suspension storage an independently gated stage before routing
general JS calls. The 12–20 week figure is optimistic if it excludes T074 repairs and
full hook/suspension conversion; use the range above with explicit uncertainty.
The counts strongly support early call/env work but do **not** prove which
aggregate wall contract any stage can meet, or that retiring wide is cheap.
