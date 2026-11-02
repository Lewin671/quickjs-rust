# S3c retirement audit

Audited runtime content **2aa117b7**, with source hashes in the
[semantic seal](semantic/source.json). This applies the original inventory
through lead decision 2's revised ABI; it does not claim the superseded
boxed-per-call directory design was implemented.

| Old surface | Final source state |
| --- | --- |
| GeneratorSnapshot, into_snapshot, field reconstruction | No definitions/references; whole boxed state survives from preparation |
| GeneratorStart and four literals, late input-seeding resume API | No references; four production entries call prepare_suspended_start |
| Duplicate suspended bytecode Rc | Removed; the frame retains its code handle |
| ActivationStack, two tier TLS register owners, depth rebinding | Removed; one FrameStorage definition in frame_bank |
| released_activations and four per-callee reuse helpers | No references; cleared capacity has no callee/name eligibility |
| release_for_reuse / reuse_for_call | No references; Register Context is materialized only on semantic demand |
| OperandStackRecycler / LocalSlotRecycler / ColdFramePool / ir_pools | Deleted; register/general free capacity shares one budget |
| Per-op scope/reference/lifetime polling | Replaced by compiler-relocated BindingBoundary operations |
| Repeated checked assignment on proven captures | Link-validated Var/simple-parameter routes use the same retained cells |
| Multiple FrameState literal builders | One literal, in vm_frame_init |

General synchronous frames remain owned values, as decision 2 requires;
suspendable frames select Box storage statically. There is no per-op ownership
tag or directory/base lookup. Source searches find zero occurrences of the old
types/protocol names above. These are static deletion facts, not timing claims.

Captured-write exclusions remain: merely initialized lexicals, imports, NFE,
object-route cells, default-parameter TDZ and extensible lookup. General/global
stores retain their checked semantic routes. Cells/arguments/scope owners have
independent identities; pooling clears private values and retains no Context,
code, receiver or selected reference. Cold capacity charges its retained vectors.
All seven suspension kinds keep the same box; native re-entry holds no pool
borrow and native argument slices pin the detached active storage.

Deferred by the approved boundary: S3d's general continuation driver, effectful
hook routing, dormant-driver/nested-Vm retirement and single stack guard; S3e's
universal Code lowering and removal of current algorithm-specific streams.
Those retained paths are not claimed as S3c deletions. No CLI stack policy,
dependency, Rust async, FFI, unsafe or benchmark-shape recognizer is added.
