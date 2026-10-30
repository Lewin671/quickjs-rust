# T077 S3a implementation and verification

Owner branch `agent/t077-s3a-native-entries/codex`, base `d19d3aa6`.
Implementation present; full check and NG fixtures pass; exact/counter gates pending. No push, merge,
iMac timing or held-out input access. Lead owns formal aggregate acceptance.

## Replacement and API

All 592 NativeFunction variants have exhaustive installation binding (584 without
agents), including internal reactions and a throwing TDZ-marker entry. Each entry
selects one operation directly; shared semantic helpers still live in their family.
NativeFunction identity remains for guards/semantics, not ordinary dispatch.

The source ABI is `fn(&mut CallEnv, CallInfo<'_>, &[Value]) -> Result<Value,
RuntimeError>`. CallInfo borrows callee, receiver, optional new.target and active
realm; CallEnv supplies the current caller services. Native VM call/construct
borrows from a detached owning operand vector, restored before catch/finally.
Host/callback and compact/wide tier entries borrow slices. Semantic retained
arguments, bound lists, Proxy trap arrays, apply/spread/rest lists still own values.

## Small corrections and measured costs

- The current interpreter has no explicit Context/activation directory. Use its
  existing OperandStack take/replace owner; S3c/d can replace that owner without
  changing the native ABI. No new driver or backend is introduced.
- Checked native re-entry needs one realm-local depth cell in `function/env.rs`.
  It is shared by callback frames, releases on return/throw, and limits synchronous
  native depth to 32. No global mutable state or RefCell borrow spans callbacks.
  This is the only service-state addition; S3b still replaces CallEnv directly.
- `FunctionData` grows from 296 to 304 bytes on aarch64: the direct entry pointer
  replaces a one-byte family cache. Its explicit size test now admits that word;
  the cold-header and lazy-map assertions remain. No new dependency/crate/unsafe.
- Constructor prototype lookup could previously exit before restoring new.target.
  The common slice constructor now restores it on that error as well as on callee
  return/throw. The focused test preserves exception identity and outer new.target.

## Verification in progress

At the worktree based on `d19d3aa6`, nine `native::tests` pass: retained host and
bound values; nested native/JS callbacks; callback throw and caller operands;
async reaction retention; distinct-realm construction with explicit new.target;
prototype-getter throw; shared depth/error release; TDZ marker.
The first whole-runtime run had 2,381 passing tests and only the deliberate
header-size assertion failure (296 budget versus the new 304); that assertion
was updated with the direct-entry cost. Final full results follow below.

Before counters: the original retained instrumentation patch builds successfully
on `d19d3aa6`; all ten development checksums/stdout and two complete counter maps
per program reproduce [the inventory](T077-s3-counts.md) exactly. In particular:
86,869,971 family probes, 78,693,537 declines, 10,986,795 family/core calls,
7,438,472 pop-argument vectors, 448,836 tier-fallback vectors and 5,259
slice-fallback vectors. These are construction events, not malloc or time.

On the implementation worktree, `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`
passes: default clippy, agents/perf-counters clippy and selected feature tests,
workspace tests (2,391 runtime tests), benchmark/script/context/size gates and
5,170 curated Test262 cases. `./scripts/compare-qjs.sh` passes all 257 fixtures.
The first full run preceded a mechanical constant-fold simplification in two
async-generator entry adapters; the pre-commit fast gate verifies the final source.

The subset honors TEST262_DIR. The exact-scan scripts hardcode their local
submodule path, so both pinned top-level submodules were initialized in this
worktree (no nested submodules, no main-checkout edits, no harness/config edits).
