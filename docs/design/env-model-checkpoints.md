# Binding checkpoints in executable IR

`bytecode/binding_boundaries.rs` inserts BindingBoundary before code is shared or
any tier/recovery map is built. Empty blocks, catches, loops and joined function
bodies may use their parent's PC scope only when they own no declarations, with
object or separate variable target. Descriptors and binding identities stay live.

A transient relocation updates jumps, handlers, prologue/exposure PCs, reference
intervals, and all PC-bearing Ops together. Scope PC entries cover the final
stream. ReferenceEvent.pc names the actual operation; its preceding checkpoint
holds the event index. Source byte spans are unchanged. Tier CFG/stack/recovery
maps derive from the final stream, never from obsolete instruction offsets.

Plain interpreter operations poll no scope/reference side tables. A checkpoint
updates scope, selects/reads/writes a reference, and cleans up at lifetime ends,
abrupt edges and handler entries. Selected bases survive RHS effects and
suspension. Promotion before selection restarts at the checkpoint; completed
effects restart after it. Wide start/cleanup uses one explicit cold operation.

A tier may omit a checkpoint only under its existing scope/reference admission
proof. Deferred globals still require a pure, bound route at entry and publish
through the canonical GlobalEnv path. New checkpoint Ops do not themselves prove
purity. See [binding authority](env-model-rewrite.md) and
[cell capabilities](env-model-cell-policy.md).
