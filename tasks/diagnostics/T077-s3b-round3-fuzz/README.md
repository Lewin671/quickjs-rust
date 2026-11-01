# Round-three seeded differential triage

Runtime `f6f5b66e`, tool `e7a6c0e4`, base `3cc12c6f`. Initial 6,000 profile/seed
programs complete in all six modes with stable source/binary/producer hashes.
No panic, crash, timeout or candidate-mode disagreement. This is **not clean
qualification**: [summary](initial-summary.json) and [new-difference inventory](initial-regressions.json)
retain PRE/CHANGED/REGR results, including partial repairs and reference defects.
[Metadata](initial-metadata.json) binds the raw request/cases. Profiles each have
2,000 seeds; classifications can overlap in one program.

## Annex B publication timing

A reduced new difference is `var f;try{throw 0;function f(){}}catch(e){}typeof f`:
candidate returns function, base/NG return undefined. Block function instantiation
must initialize the lexical binding early, but Annex B's outer-variable copy runs
only when evaluation reaches the declaration statement. Existing regular blocks
had the same defect; repairing try/finally made it newly reachable there.
The repair separates lexical instantiation from the declaration-time copy across
blocks, if clauses and the existing switch statement path. Root function-body
instantiation and strict/generator/async declarations do not gain Annex B copies.
Tests cover skipped declarations, early lexical calls, outer reads before/after
publication, with and finally. The focused test fails before the repair.

## Reference differences retained for spec triage

Pinned Test262 `language/statements/with/get-binding-value-idref-with-proxy-env.js`
and its call/compound variants require HasBinding then GetBindingValue to perform
two HasProperty calls. NG (and Node's optimization) often reports only one in
these generated routes. Candidate follows the pinned test's protocol; no patch
will reduce its observable calls toward the reference's incomplete trace.

A minimal destructuring-catch initialization failure must still run finally:
`try{try{throw undefined}catch([x]){}finally{log.push('finally')}}catch(e){...}`.
Rust and Node run finally; pinned NG skips it. The raw generated programs keep
these differences; output normalization does not remove them.
