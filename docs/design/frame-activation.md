# Frame activation (superseded proposal)

The 2026-10-29 lead/user decision replaces the T056 cohort/driver migration with
[core rebuild execution](core-rebuild-execution.md) and
[T077 S3](../../tasks/T077-core-rebuild.md#s3--universal-bytecode-frames-calls-and-native-abi).
The new contract uses zero-based per-activation blocks, one driver, direct
arguments, typed context and native slice ABI. No shared-window per-access base
addition or retained legacy CallEnv cohort is authorized by this old proposal.

Historical cost/cohort inputs remain in
[T056 M2 evidence](../../tasks/archive/T056-m2-design-inputs.md) and
[T056 plan inputs](../../tasks/archive/T056-m2-plan-inputs.md).
The old design is preserved in Git before this change.
[Binding](env-model-rewrite.md) and [suspension](generator-suspension.md)
invariants remain authoritative for implemented semantics.
