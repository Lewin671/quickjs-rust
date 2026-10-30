# Interpreter core v3 (superseded)

The 2026-10-29 lead/user decision replaces this unimplemented proposal with
[core rebuild](core-rebuild.md) and [T077](../../tasks/T077-core-rebuild.md).
The proposed alternate backend, Copy words, precise tracing GC and bulk builtin
port are not the target. Use in-place replacement, owning RC values and a
separate backup cycle collector; implementation follows T077's stages.

The old proposal is preserved in Git before this change. Historical evidence,
review and the superseding decision are in
[T065](../../tasks/archive/T065-interpreter-core-v3.md).
Current implemented behavior remains [architecture](../architecture.md).
