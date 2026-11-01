# Rejected merged S3b exact checkpoint

Complete cached scan of runtime `73c93d8f`: 53,572 total, 10,900 unchanged
configuration skips, **42,669 Rust passes / 3 failures / 0 timeouts / 0 not-run**.
NG: 42,602 passes / 70 failures / 0 timeouts. Three actionable gaps; no
correctness acceptance. [Summary](summary.json), [failures](failures.json),
[source, command and raw-file hashes](metadata.json). Its complete burndown row
is appended without modifying earlier records.

The bound-function realm failures expose the synthetic host shim's constructor
function used for `%Function.prototype%`: it has an own `prototype` property.
Typed creation metadata now correctly supplies that selected function prototype
to bound wrappers, making `bound.prototype` accidentally inherit an object and
preventing the required intrinsic fallback. A callable nonconstructor method
preserves the shim's intended prototype shape. The real NG host uses actual
realms, not this synthetic shim. No exclusions or expected failures are added.

`script-decl-lex.js` exposes a real binding regression: a free-name sloppy write
after host-script lexical declaration takes the old realm-cache publisher. The
checked publisher intentionally cannot modify declaratives, so the lexical cell
keeps its previous value. The repair selects that canonical cell before the
object-cache route, preserving lexical mutability, TDZ and const checks.

The queued IC-off scan began after cached completion and was terminated by its
owner during discovery of this rejected source; it has no complete result or
burndown record. Both full exact modes are required again after repair/refreeze.
The prior owner receipt remains valid evidence for its named checks, but does
not establish complete correctness or S3b acceptance.
