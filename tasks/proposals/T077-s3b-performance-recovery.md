# T077 S3b round-4 recovery plan

Frozen source: **7b3ed258**, runtime/order **e1405670 / 8413f925**; performance
baseline **3cc12c6f**. Authority: [lead feedback](../diagnostics/T077-s3b-r4/lead-feedback.md),
[independent attribution](../diagnostics/T077-s3b-r4/attribution.md). This is the
ordered recovery of the existing rebuild stage, not a new leaf-unit attempt.
All round-1–3 fixes/tests and the typed Context/slot/shared-cell model remain.
No caller-environment substitution, name/value snapshot or apply-back returns.

## Ordered migration

1. **Reference admission and canonical global sinks.** Cache plan admission by
   actual realm, global lexical epoch and binding/property structural identity;
   value writes do not invalidate a proof. Sloppy/bound-strict global sites use
   compiled checkpoints, not an all-op lookup. A checkpoint proves a bound global
   before RHS effects, or saves its selected reference in the existing positional
   bank. Missing strict names retain unresolvability. A global lexical epoch
   advances only on new lexical declarations; affected live intervals retain their
   pre-effect base, including across continuation/VM exits. Wide execution gains
   these checkpoints rather than rejecting the entire body; typed loops skip only
   with their existing no-user-code/type proofs. Canonical global sinks accept the
   exact current realm cell without requiring a copied local value.

   This refines the investigator's proposed post-call pinning: a lexical epoch
   alone cannot reconstruct initial resolvability after earlier RHS delete/add
   effects. Selection/proof stays at the actual LHS boundary. Shared continuation
   reference state contains references and compiled indices, never name/value data.
   Cache proofs and cold hooks are activation-specific when invalidated; no old
   answer is replayed and no observable Has/Get/coercion is duplicated.

   Concrete ownership: one lazily allocated, index-addressed SavedReferences bank
   belongs to ActivationContext and is shared only by its continuations. VM and
   wide execution use the same selected references across exits; generator state
   retains it through the activation. Immutable reference events lower into wide
   checkpoints; a realm-epoch change pins live elided (proven-bound) global sites
   before their next read/store. An originally unbound site already has its saved
   unresolvable reference and is never retroactively re-resolved. Global proof
   caches retain weak realm identity and structural metadata, never settled values.
   Dynamic-scope sites keep full ordered resolution. Cold reference-bank allocation
   is separately counted; ordinary calls with no reference demand allocate none.
2. **Lazy indexed call state.** Keep one canonical call owner while reducing the
   ordinary inline record to demanded call metadata. Defer identities/cold scopes,
   borrow callee creation inputs where sound, avoid owner cold borrows through a
   compiler/function fact, and build no activation for a numeric answer. Materialize
   full state for VM/native/exposure, preserving receiver/new.target/home/error realm
   and continuation identity. Mutable initialized declarative capture writes use an
   explicit checked admission/proof; TDZ/const/import/object routes remain checked.
3. **Live-scope admission.** Prove no with object and no intervening holder binding
   for the function's compiled free-name demand. Cache by holder structural epochs;
   check at entry and re-entry, deoptimizing before stale routes are used. A value
   write is not a shape change. Selected pre-RHS references retain their old base.
4. **Immutable leaf effects.** Compile numeric-plan purity once and hoist/reuse the
   typed-loop call context across admitted calls. Keep all effect/throw/replay fixes.
5. **VM dispatch and globals.** Consolidate rare scope/reference/authority work into
   explicit transitions or one cold-flags check. Cache canonical global routes by
   structural identity, retaining descriptor/proxy/deletion/lexical semantics.

Before editing each stage, confirm its concrete module ownership and invariants;
record any better representation with its rationale. Split large existing modules
by semantic responsibility rather than exceed file budgets. No new dependencies,
unsafe, FFI, Rust async or benchmark/source recognizers are planned.

## Evidence and gates

- Add diagnostic reason counters/traces for reference admission, indexed-binding
  admission and hook-blocked loop edges; distinguish cache proof hits/misses and
  cold/reference materialization from actual execution entries.
- Reproduce deterministic local ARM instruction counts with `/usr/bin/time -l`,
  three repeats, hash-pinned ordinary release base/candidate binaries and unchanged
  [reproducers](../diagnostics/T077-s3b-r4/repro). Record instruction counts only;
  local wall times are not acceptance evidence. Collect corresponding perf-counter
  maps separately; diagnostic binaries are never timed.
- In order: focused semantic tests and staged gate per reviewable code unit, then
  public86 after cause 1 and after cause 2 through the iMac queue. Retain every run.
- After all five: freeze source, run the full gate, seven-mode NG/module matrix,
  individual/combined stress, public correctness, >=2,000 seeds per differential
  profile with triage, complete cached/off exact scans with separate complete
  burndowns, depth parity and deterministic development counter maps.
- Regenerate the default 76-case x86 order and run final public86. Each lane must
  be **<1.00 wall and instructions** versus 3cc12c6f. If any still misses after all
  five causes, classify every affected case and stop; no further tuning loop.
  Lead owns formal, held-out, RSS, re-review and acceptance.
- iMac status/date first; no timing at UTC 18:45–21:00. Runner ref transport is
  allowed. No origin push or merge into main. Update T077 resume/progress before
  handoff and commit every coherent verified unit on this owner branch.
