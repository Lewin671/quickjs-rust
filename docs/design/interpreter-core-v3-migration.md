# Interpreter core v3 migration (superseded)

The 2026-10-29 lead/user decision replaces the unapproved dual-backend M0–M5
route, supported-slice milestones and its budgets with
[T077 S1–S7](../../tasks/T077-core-rebuild.md) and the
[core rebuild target](core-rebuild.md). Every replacement stage preserves the
complete configured Test262 zero-gap floor; no old-engine fallback is retained.

The old migration proposal is preserved in Git before this change; its review
and unimplemented status are in [T065](../../tasks/archive/T065-interpreter-core-v3.md).
There is no M0 or bulk builtin-port authorization from this document.
