# T077 S3c: owning activations and priced S3b recovery

- Status: design and inventory only; awaiting lead approval; base `8322fc98`.
- Implementation: absent. No runtime, compiler, Cargo or production script change.
- Evidence: [inventory](T077-s3c-inventory.md), [counting receipt](../diagnostics/T077-s3c-counting.md), [records](../diagnostics/T077-s3c-counts.json).
- Authority: [S3 split](T077-s3-plan.md), [execution target](../../docs/design/core-rebuild-execution.md), [protected bindings](../../docs/design/env-model-rewrite.md), [S3b acceptance](../diagnostics/T077-s3b-acceptance.md). The acceptance receipt supersedes the split's neutral-S3c allowance.
- Next action: lead approves the ABI, recovery scope and migration below before runtime work. No push or integration is authorized by this design.

## Decision

Storage ownership alone is not a credible payer for S3b's roughly 3% loss.
The development corpus has no suspension reconstruction, and most calls already
use a register tier without a general Vm. Retaining another pool while leaving
S3b's binding protocol in those tiers would miss the purpose of this stage.
Adopt the ownership ABI below across existing executors, remove their eager
binding/release protocol, and add the two binding-policy cuts below. The counted
payoff and the smallest proposed S3d pull-forward are in the receipt; estimates
are instruction budgets, never a wall forecast or permission to weaken a gate.

## Ownership ABI

`Executor` owns a directory `Vec<DirectorySlot>` and one cleared-storage pool.
`FrameId { index:u32, generation:u32 }` is 8 bytes, scoped to an Executor token.
A slot is Vacant, Parked(Box<Activation>), Running or Suspended; only Parked
contains a box. Running and Suspended reserve an ID but cannot be dereferenced.
Generation advances on retirement, never on suspension; on u32 exhaustion retire
the slot permanently. Capacity overflow returns a structured resource error.

An activation owns its program, one boxed register block, optional exposure
state and cold semantic state. At S3c the block contains the existing locals,
operand and tier-register vectors plus init/cell-route maps; it is not a global
contiguous register stack. S3e performs the single conversion to `values:Vec<Value>`.
No local is addressed through `store[base+r]` or a per-access directory lookup.

`detach(id)` takes a Box into `CurrentActivation`; the driver binds code, PC,
local/operand/register slices and call inputs once per non-reentrant segment.
A switch ends those borrows, stores PC/result destination and cold continuation
ID fieldwise, parks the old box and takes the next box. Move pointer owners
(8 bytes each), FrameId (8) and scalar PC/destination; never move/copy the current
448-byte FrameState or the 464-byte snapshot. No aggregate Action contains an
activation. Code views must end before changing execution authority or ownership.
The existing nested synchronous call structure stays until S3d; it borrows these
owners through a scoped boundary instead of owning a FrameState by value.

### 64-byte header and call fields

Three inline owning Values would require 48 bytes alone. Use typed **slot
references in the header**, with sole owning Values in a fixed call prefix of
the block; this is the explicit representation choice for lead approval.

| Field | Bytes | Contract |
| --- | ---: | --- |
| program owner | 8 | Rc<Bytecode>, later Rc<Code>; immutable owner independent of caller |
| block owner | 8 | Box<RegisterBlock>; stable allocation |
| context/exposure owner | 8 | optional Box; realm/creation is borrowed from retained callee until exposure |
| cold state owner | 8 | optional Box; handlers, references, disposal, resume and partial classes |
| receiver, new_target, home | 12 | three typed u32 CallSlotRefs, never JS binding names |
| continuation | 4 | typed cold-record index, sentinel for host boundary |
| PC, return register, actual argc, flags | 16 | four u32; checked bounds/conversions |
| Total | **64** | repr(C); assert size/alignment/offsets on arm64 and x86_64 |

Callee ownership moves into a fourth fixed block-prefix slot, replacing the
target sketch's separate callee handle. An ordinary receiver is read directly
from its bound prefix slice. CallSlotRef distinguishes Absent, InlineSlot,
SharedOwner and DerivedCell; new_target/home absence is distinct from undefined.
Lexical promotion moves the sole call owner into Rc and redirects all routes;
no inline mirror remains. Derived this remains one TDZ BindingCell. An arrow
uses its retained owner's routes; ordinary functions install fresh routes.
Method/home/super and constructor state remain canonical even when callbacks
materialize Context. Sloppy receiver coercion still uses the callee's realm.

Root builders retain compiled bodies in Rc once per host entry; do not clone
bytecode on calls or resumes. This adds an unpriced root ownership conversion
at APIs currently borrowing Bytecode: count and inspect it separately, including
small-script startup. Existing borrowed-code views are segment-local only.
`Context` remains a borrowed service/view, now over header routes and optional
exposure payload. Materialize identity/scopes/holders only on demand; ordinary
inline calls do not construct/rebind a 120-byte ActivationContext. The ABI must
work for all language features, including wide exits, before any old owner is removed.

### Pool, lifetime and re-entry invariants

One Executor pool, at most **64** cleared activation/block pairs, **256 KiB** total
retained capacity (including headers, allocation capacities, bitmaps and route
arrays), **16 KiB** maximum per pair. Capacity size classes avoid name/function
matching; on overflow drop the pair. Active, native-borrowed and suspended blocks
are outside the free pool and never evicted. No new recursion limit is introduced.
Initialize each admitted parameter, hoisted var, TDZ bit and call-prefix field
fieldwise; track written ranges so clearing does not scan unwritten capacity.
Only proven temporary slots may be consumed; coercion must not empty a local.

Cells, lexical owners, scope leases, eval holders, mapped arguments and closures
have independent owning identities. Promotion moves a slot to its cell and
clears it; every route thereafter reads the same cell. Loop/catch re-entry makes
a fresh cell. A lease/arguments object/closure never points into an activation,
FrameId or pooled block. A returned exposed frame may donate **cleared storage**
only after dropping its unique metadata and transferring all escaped owners;
exposed identities and cells themselves never recycle. No name snapshot/apply-back.

A native boundary owns the detached caller box outside the borrowed directory;
its argument slice is pinned until return/throw. Release all directory, services
RefCell and cell borrows before JS re-entry. Callbacks allocate/take other boxes.
Rest, spread, bound lists and retained arguments still own their semantic values.
A Context or slice cannot escape a segment; jobs retain owned data only.

Suspension consumes CurrentActivation into a ticket owning the **same box and
block**, FrameId/token, PC, resume kind and cold state. Its directory entry is
Suspended, with no duplicate box. Resume consumes the ticket, marks Executing
before running JS, then reattaches that same block; no frame builder, slot seed,
mask reconstruction or GeneratorSnapshot is run. Suspension in another Executor
retires the source reservation and registers a new ID while preserving box/cell
identity; FrameIds are not global references. Dropping a never-resumed ticket
retires its reserved ID and drops owners without running JS. Directory bookkeeping
must not retain the ticket (no new Rc cycle). Host jobs carry runtime-group/realm
ownership, not borrowed RuntimeServices or a caller Context.

Keep generator prologue-at-call, async rejection timing, yield* resume variants,
async-generator FIFO/adoption, TLA module cells, pending finally completion,
iterator-close/disposal and partial class state. Ordinary return never drains jobs.
Abandonment does not execute finally. Existing Rc-cycle collection limits remain.

A ticket retains a directory reservation owner, not the Executor driver or a
Context borrow; reserved entries contain no backlink to tickets. Executor teardown
can leave this small reservation registry alive until outstanding tickets drop.
Directory high-water capacity is a separate reported RSS item, not pooled bytes.

Debug checks directory state/generation, one-box ownership, header routes, exact
written/init masks and no dual slot/cell authority at every boundary. Pool poison
fills cleared value slots and stamps generation/route maps; checkout verifies it
before fieldwise init. Existing QJS_VERIFY_BINDINGS, QJS_FORCE_EVAL_EXPOSURE,
QJS_POISON_RELEASED_SCOPES and QJS_POISON_RELEASED_ACTIVATIONS retain their coverage;
activation poison now also covers blocks, suspended tickets and boundary return.
No poison reads affect release code. Test stale IDs, alternating realms/methods,
exposed frames, GC-like owner drops, errors and suspended finally before pooling.

## Removal of the three S3b obligations

1. **Call binding:** compile immutable entry facts; initialize header call routes
   once, borrow creation from callee, and create exposure state only when needed.
   Delete same-function slot rebinding, release eligibility scans, the out-of-line
   activation push wrapper and Context depth rebinding in both register drivers.
   Keep actual receiver coercion/argument/prologue work. Budget **30–60 residual
   instructions** against the old **130–150** protocol: **70–120 saved** on a
   proven unexposed inline entry. This is a net target including directory/pool
   traffic; counters cannot price the new boxing cost before implementation.
2. **Captured writes:** BindingPlan/capture linking supplies an InitializedMutable
   route for var/parameter cells with a proved completed initialization. Static
   metadata chooses write policy, linking validates route kind once; use one safe
   RefCell replace without repeated metadata/TDZ/strictness checks. Never bless a
   merely observed initialized lexical, import, NFE name, object-route cell,
   parameter default TDZ or extensible dynamic lookup. Unproven writes retain
   checked Assign. Debug validates capability at link/store. Budget **25–45**
   instructions per proven write versus about **125**, saving **80–100**.
   No unsafe operation and no relaxation of the protected cell model is needed.
3. **Hooks and eval locals:** emit explicit scope/reference start/read/write/cleanup boundary Ops in the
   existing IR, remapping jumps, handlers, source PCs, reference intervals and
   every tier recovery map atomically. Side tables supply the boundary metadata;
   a dispatcher must not poll them on every ordinary opcode.
   Current general Op remains executable until S3e. Bound globals still call
   set_bound_global (declarative first, then full object record); an unresolvable
   start retains its exact selected base through RHS and suspension. Dynamic
   with/unscopables still resolves observably in order. Never use the investigator's
   semantically wrong experiment-A patch or a global epoch alone as proof.
   Own eval-exposed static locals get indexed cell routes at promotion; no name
   or slot-policy discovery per read, no value cache. Holder add/delete and
   parameter/body/with joins invalidate only dynamic selection, not cell identity.
   Target **15–30 instructions saved** per measured direct-cell slow read, plus
   **5–10** per checking-loop iteration and **5–15** per reference event whose
   polling/lookup is genuinely removed, net of the inserted boundary dispatch. These
   are unvalidated net budgets; event count is an upper bound, not all removable.
   Plain opcodes pay **zero reference/scope polling instructions**; indexed
   initialized-cell read bookkeeping targets **10–25 instructions** plus the
   required Value ownership operation. Semantic reference resolution is still
   charged at its explicit boundary, never assumed free.
   Checkpoints lower through the current VM/tiers now; S3e owns final Code opcodes
   and verifier, not recovery of S3b's checking-loop overhead.

## Migration and acceptance

See the [inventory](T077-s3c-inventory.md#migration) for ordered complete commits,
files/deletions/gates, ownership ABI freeze and explicit S3d/S3e boundaries.
Each checkpoint preserves all existing language support; a failing cut is repaired
or reverted, not landed with an unsupported cohort or a new fallback backend.

Freeze base **8322fc983da20800ce612ad148781aea30e28d08**. On the iMac, formal
development, external, equal-case selection and sealed held-out must each be
**<1.000 against that base**, with the existing precision, case-health and layout
protocol. External additionally **<=0.983** against main (about 1/1.017; collect
a direct fixed-rebuild-baseline comparison, do not accept multiplied ratios as
proof). Report NG and cumulative milestones. RSS **each case <=1.10**, geomean
**<=1.05**; both RSS limits are required by this S3c brief.
No local timing and no held-out diagnosis/scope selection in this design.

Correctness is the S3b final standard: focused ownership/binding/suspension tests;
full check; default/agents/perf-counters on arm64 and x86_64; seven stress/poison
modes plus combined IC-off; compare-qjs in all nine modes including modules;
public correctness sweep; tools/binding_fuzz **>=2,000 seeds per profile**
(binding, binding-simple, receiver, global-store), with no new unexplained
regression or mode disagreement; complete exact Test262 cached and IC-off,
zero gaps/lost passes/new exclusions, burndown after each complete scan.
Release **and debug depth >=8322fc98** wherever frame layout changes. Restoring
S3b's earlier debug deficit against 3cc12c6f remains S3d's separate obligation.
Inspect optimized assembly: zero-based slices, no per-op FrameId/base resolution,
no several-hundred-byte switch copies or Action spills. Poison and counters are
never timed. Freeze candidate before formal/held-out acceptance and stop on a
failed aggregate; no promise of S3d/S4 can authorize a losing S3c landing.

The current counted instruction envelope is below the external recovery target;
see the receipt for all four selectable lanes and the unpriced held-out panel.
The 3–5 week estimate covers the specified S3c work, **not** a demonstrated 3%
payer. A call-only pull-forward cannot be credited with already-deleted argument
vectors. Lead must choose a bounded standalone trial under unchanged gates or
re-freeze the combined S3c/S3d landing before implementation; no weakening of
the performance contract is proposed.
