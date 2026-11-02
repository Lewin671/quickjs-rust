# T077 S3c: source inventory and complete migration cuts

Design-only companion to the [brief](T077-s3c-brief.md); audited at `8322fc98`.
Runtime paths below are relative to `crates/qjs-runtime/src/`. Historical
[initial inventory](T077-s3-activation-inventory.md) described `dd0905b2`, before
S3b; its CallEnv sites are not current construction authorities.

## Present ownership and proposed deletion ledger

Sizes are local arm64 rustc 1.95.0 type-layout output from unchanged main,
not stack-frame sizes or x86 measurements. Reproduction is in the
[counting receipt](../diagnostics/T077-s3c-counting.md).

| Current owner / exact surface | Replacement / deletion at S3c |
| --- | --- |
| `vm.rs::FrameState`, **29 fields, 448 bytes**; one production struct literal in `vm_frame_init::with_frame_bytecode` | one fieldwise activation builder; header/block/cold ownership, no by-value frame ABI |
| `vm.rs::ColdFrame`, **14 fields, 288 bytes** | retain semantic fields as owned cold state, not a completion snapshot; handler/disposal/reference state is not deleted |
| `vm_generator::GeneratorSnapshot`, **19 fields, 464 bytes**, one literal factory `into_snapshot`, one restore function `run_from_yield` | delete snapshot type, export factory and restore constructor; nine `.into_snapshot(...)` call sites become ticket suspension |
| `GeneratorStart`, **4 fields**; **4 production literals**: `function/call`, `async_function`, `async_generator`, `vm_module` | the same builder yields an owned Start activation; initial input/prologue policy stays distinct from resume |
| **3** generator-body Vm constructions: prologue, start, resume | prologue/start build once through the common owner; resume never constructs/re-seeds a Vm; seven SuspensionKind variants and all Resume variants survive |
| `vm.rs::Vm.current`, `frame_stack::SuspendedFrame.frame`, pending `(FrameState, FrameContinuation)` — **3 by-value owning fields** | current detached Box and IDs/tickets; replace `push_frame(FrameState)`, `into_frame()->FrameState`, caller assignments/whole-frame `mem::replace` protocol |
| `compact_fn/activation_stack.rs::ActivationStack`, one type with **2 driver owners**, 64 parked slots / <=16 KiB per run | delete file/type, both callee-slot/release paths, both Context `rebind_env!` depth macros; common activation storage instead |
| `RuntimeServices.released_activations`, **16 activations / <=4 KiB** | delete services pool; no cross-run same-creation eligibility matching |
| `function/call::{reenter_indexed_activation,released_indexed_activation,recycle_callback_activation,keep_released_activation}` — **4 reuse functions** | delete; `enter_indexed_activation` and `bind_indexed_call_owner` become the lazy entry/header builder, not compatibility wrappers |
| `ActivationContext::{release_for_reuse,reuse_for_call}` — **2 protocol methods**, current context **120 bytes** | delete eligibility/rebind machinery; exposure payload is unique, pooled storage cleared independently |
| compact and wide `FrameStorage`, **2 TLS caches**, each retaining <=4096 registers; CompactFrame **80 bytes**, WideFrame **48 bytes** | delete their register/window ownership and ActivationStack, consume common per-activation blocks; algorithm-specific PCs/admission remain until S3e |
| `OperandStackRecycler`, `LocalSlotRecycler`: each per body <=32 vectors, capacity <=256; `ColdFramePool`: <=8 boxes | delete per-body value-vector recycling and independently owned operand bank; move cleared block/cold capacity under one counted pool budget |
| `FrameBytecode` / `FrameProgramView` | keep program ownership and segment-view invariant; root builder acquires Rc once; delete borrowed lifetime propagation from suspended activation storage |
| `vm/wide_resume.rs` builder and state translation | same activation owner on tier/interpreter handoff; retain exact-PC/recovery translation until S3e |

Counts are definitions/owning fields/call sites, not all textual mentions or a
promise to delete the whole 3,225 lines in the five original S3c core files.
The snapshot's fields and cold state must all have a destination; moving a
field out of sight is not a semantic deletion. Current snapshot reconstruction
also recomputes scope/authority masks and creates/replaces locals/stack/cold
storage; same-block resume eliminates all those restoration operations.

## Lifetime/consumer inventory

| Consumer | Required identity and complete conversion |
| --- | --- |
| `vm_frame_init`, `vm_import`, `vm_jobs`, `global`, `vm_module`, `module/{link,mod}` | root/eval/module/host entries acquire body and activation ownership; jobs/referrer/agent/can-block remain typed services |
| `function/call*`, `function/arguments*`, `function/upvalue*`, `context/{activation,activation_api,call_owner,services}` | parameter/body split, actual argc, lazily exposed arguments and one lexical owner; Context borrows the selected header, never caller equality |
| `bytecode/{binding_plan,upvalue_resolver,vm_bindings,vm_scopes,vm_references,reference_plan,reference_admission}` | static capture policy/indexed cells; dynamic reference intervals retain selected base, cell and holder identities through calls and suspension |
| compact/wide activation and entry/exits; typed and numeric-mutation loops | same block/owner routes for fast bodies, recovery maps retain slot/cell semantics; algorithm-specific execution survives |
| `generator`, `async_function`, `async_generator`, `vm_generator`, `vm_module`, promise reactions/jobs and `value/object` generator slot | start/Executing/suspended/completed transitions, same-box ticket, async FIFO/adoption and TLA cells; no borrowed service owner in a retained job |
| `vm_try`, `vm_dispose`, `vm_class_region`, `vm_result` | pending abrupt completions, try/disposal/class depths, delegation resume mode; restore exact semantic step before running handlers |
| native entry/callbacks, coercion/property/iterator helpers | detached caller/immutable arguments across re-entry; no live directory/RefCell borrow; full continuation routing remains S3d |

## Migration

Each row is a reviewable commit or tightly bounded complete commit series on
one implementation branch. Every intermediate commit runs all currently
supported language features; focused cuts do not justify a partial-support
landing. The final S3c commit is the performance acceptance boundary. There is
one production owner per execution path throughout, no new backend flag.

| Cut | Files / replacement and deletions | Gate before advancing |
| --- | --- | --- |
| C0 independent class fix | `bytecode/vm_class_region.rs`, `function/value.rs`, class tests: initialize the fresh constructor's internal prototype through the existing explicit-prototype constructor authority, avoiding the generic cycle walk; retain checked mutation for existing functions | NG class heritage/order/prototype fixtures, focused class tests, deterministic traversal counts N/2N/4N linear, fast gate; do not time |
| C1 owning general frames | split `vm.rs` storage into activation/header/block owner modules; `vm_frame_init`, `frame_program`, `frame_stack`, `operand_stack`, root/eval/module builders; replace sole FrameState literal and 3 by-value fields/transfer API; common bounded pool replaces per-body vector recyclers | root/borrowed-API startup ownership, retained cells, arguments, errors/finally, eval, cross-realm callbacks and dormant-driver tests; stress+poison; fast/full gate; verify type layouts and assembly; every builder uses the owner |
| C2 register consumers and lazy Context | compact/wide `activation{,/entry,/exits}`, `function/call`, `context/*`, loop consumers and wide-resume; remove ActivationStack file, 2 TLS register owners, services reuse pool, 4 reuse helpers, 2 eligibility methods and 2 depth-rebind macros | ordinary/method/arrow/construct/native re-entry and retained-owner alternation; all public counter maps/checksums, upw0/upw1/upw4 N/2N slopes, no eager context per unexposed entry; debug/release depth >=base; compare all modes; no new directory/base load on opcode backedges |
| C3 static binding routes and event boundaries | BindingPlan/capture linking, `binding/cell`, compact/wide store plans, `vm_bindings`, `vm_scopes`, `vm_references`, reference plans/admission, explicit boundary Ops and all PC/recovery maps; delete proven mutable-write checks and per-op reference polling, not dynamic resolution semantics | TDZ/const/import/NFE/default-param TDZ, with/unscopables, later eval declarations, delete/recreate and inherited/Proxy/setter globals; global-store fuzz >=2,000 seeds; all modes; counters prove fast-policy eligibility, event reach and dynamic fallback separately |
| C4 suspension owner conversion | `vm_generator`, all four GeneratorStart sites, generator/async/TLA drivers and job captures; remove 19-field snapshot, 9 exports and resume reconstruction; start/prologue and resume share block ABI | all seven resume/suspension kinds; yield/await in class keys/finally/disposal, shared mapped arguments/cells, cancellation/drop, re-entrancy and async FIFO; pointer/ID identity assertions across repeated resumes, poison and compare-qjs all modes |
| C5 final qualification/retirement audit | all changed runtime owners, architecture/binding/suspension documents, T077 resume/evidence and both hot order files if symbol graph changes | complete acceptance matrix in brief: full/stress/all-mode comparison/public/fuzz/exact cached+IC-off with burndown; arm64/x86 layouts and depth; then frozen iMac formal/held-out/RSS; all four improve and external <=0.983 |

**Ownership ABI freeze is after C2**, once root, general and existing register
drivers all consume the same block/header and callback borrowing contract, and
C1/C2 gates pass. Freeze the ticket state machine and tests at that point too.
Only then may a separately assigned suspension/jobs owner implement C4 in
parallel with C3 on disjoint paths; lead owns all shared fields, call/Context
APIs and integration. This proposal itself spawns no agents.

C1 uses existing ActivationContext as a uniquely owned payload; C2 removes its
ordinary-call construction and makes exposure lazy without introducing a second
context authority. C4 temporarily retains the old snapshot as the sole suspension
owner until its complete replacement; it never runs two suspension backends.
C1/C2 can be reviewed separately but cannot be declared a successful payoff stage
while C3/C4 and final acceptance are missing. Revert a failed cut rather than
retaining dormant alternative storage or an indefinitely unlandable backend.

## Boundaries, estimate and lead decisions

S3d retains the **general driver/request/semantic-continuation** architecture,
all effectful getter/Proxy/coercion/iterator/eval/disposal/class hook conversion,
**nested-JS-Vm deletion**, dormant-driver retirement, ordinary general-call argument
transport unless explicitly pulled forward below, **single runtime stack guard**
and native boundary budget, explicit CLI stack sizing, frame-counted JS depth
and restoration of S3b's **debug-depth deficit against 3cc12c6f**. C2's smaller
existing-tier entry/return sequence is not a claim those S3d jobs are done.

S3e retains universal register Code/builder/verifier, all Op lowering, common-PC
recovery, removal of executable stack/leaf/compact/numeric-plan/wide/virtual
streams and admission caches; typed/mutation scalar execution is S4-owned.
Wall-only layout gaps and diffuse codegen residuals are recorded, unpriced;
S3e/S4 own the remaining structural throughput work. No broad property/string
retuning or held-out-specific work belongs to S3c.

Base S3c is estimated **3–5 engineer-weeks**, versus the old ownership-only 2–3:
C0 <1 day; C1 roughly 4–6 days, C2 4–7, C3 3–5, C4 3–5, C5 2–4; shared work,
parallelism and conformance repairs make these overlapping sizing ranges, not
calendar promises. No new crate, dependency, FFI, Rust async, GC or unsafe code
is proposed. Keep new owner/pool/builder/route modules small by responsibility.

Open lead decisions before implementation:

1. Approve the 64-byte header's three typed call-slot references and callee prefix
   ownership, instead of pretending three Values fit inline; approve pool bounds
   and root Rc conversion. A changed header must be re-budgeted on both targets.
2. Approve the static cell policy and existing-IR reference-boundary work as S3c
   recovery scope, including initialization proof and explicit dynamic exceptions.
3. The receipt's counted envelope misses external <=0.983; a call-only extension
   has no demonstrated recovery budget. Ordinary argument Vec removal is already
   done (0 development pop_arguments calls, 90 external), so the original S3d
   inventory is not a payer estimate. The smallest extra cut worth **pricing** is
   lazy entry/return and direct parameter placement in the existing standalone
   and general entries (`function/call*`, `vm_call`, frame init); its ceilings are
   the measured owner binds outside inline calls and general frame constructions,
   **not** every tier fallback helper call. It adds roughly 1–2 engineer-weeks,
   removes the replaced prepared-entry/parameter-copy shell, preserves semantic
   spread/rest/bound lists and leaves nested-Vm routing with S3d. No savings are
   booked before the net instruction cost is measured.
4. If that bounded experiment cannot establish a credible payer, the smallest
   architecture-consistent combined landing is **S3c plus full S3d**, including
   all JS hook/construct/eval continuations: **6–10 engineer-weeks** provisionally.
   Pulling only ordinary calls into a new driver would violate the approved S3d
   cut; do not manufacture such a cohort backend. Re-inventory the now-obsolete
   argument-transport deletion and price general frame/entry costs before freezing
   the combined plan. The present evidence does **not** prove that even combined
   S3d will meet wall recovery; its full unchanged gate remains mandatory. Reject
   or re-scope a failing payer, never land another losing enabling stage on a
   promise of S3e/S4. Lead approval of this design is distinct from authorizing
   either implementation experiment or a combined landing.
