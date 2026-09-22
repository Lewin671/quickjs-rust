# T021: Single-VM Frame Stack And Compact Execution Core

## Goal

Remove recursive per-call VM construction from ordinary synchronous bytecode
calls, then compact the same execution core into register-oriented or
superinstruction dispatch. This structural T018 proposal must produce general
external wins while preserving the verified Test262 correctness baseline.
It is a foundation for the final every-case `<= 0.50x` QuickJS-NG
contract, not permission to specialize benchmark identities or loop shapes.

## Status: closed (2026-08-01)

Both structural theses were built to completion and measured. Neither is
the answer, so this task is closed; its experiments are preserved verbatim in
[`archive/T021-single-vm-frame-stack-log.md`](archive/T021-single-vm-frame-stack-log.md).
New work in this area starts from current evidence under T022, not from this
task's plan or its historical "next" entries.

### What landed

The compact register tier (`crates/qjs-runtime/src/bytecode/compact_fn/`):
admitted whole function bodies run on a compact register executor
(`815d8b79`), dispatch direct-leaf callees from registers (`a69ecae6`), read
upvalue-backed locals from their cell (`67555208`), admit only bodies whose
reads land in initialized slots (`63c775bd`), run without constructing a `Vm`
(`be43c1f5`), and hold locals in the register file (`227f7d20`).
Compact-to-compact calls also skip `CallEnv` construction once equality with
the environment they would rebuild is proven (`f30e1bf6`).

### What was built, measured and closed

These close the named implementations, not the mechanism families:

- R2 explicit single-VM frame scheduler on the direct-leaf boundary: correct,
  not promoted.
- Single-VM frame stack for the compact tier (one windowed register file for
  the whole call chain): the mechanism removed almost every standalone
  activation, yet the tier ran slower, because every register access paid a
  window-base offset. Reverted.
- Virtual stack / copy propagation in the compact compiler: correct on the
  third attempt, fewer operations, no net gain. Reverted.

### Conclusions the log supports

- On the compact tier, removing generic dispatch bought far less than
  removing per-call construction; call construction was the cost.
- The remaining gap on the tiers this task touched is the per-operation cost
  of each register-machine operation (encoding, operand access, inlining),
  not the calling convention or the value representation alone. That is a
  campaign across tiers, selected through T022, not a continuation of T021.
