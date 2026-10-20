# T063: Canonical surrogate-pair string construction

- Status: active; correctness reviewed; instruction-regression repair in progress.
- Implementation: canonical builders and staged instruction repairs present
  through `afa9c0c7`; no merge or project-remote push.
- Verified at: `8e5a2c3f`, 2026-10-20; staged fast gate, full `check.sh`, 252 NG
  fixtures, 671 focused allowlist cases, and 5107 upstream Rust cases pass.
- Evidence: durable counts and failures below; regression fixture/unit tests
  are committed. Raw logs: workspace `outputs/imac-bench/` jobs named below
  (local artifacts, not retained in this repository).
- Unresolved: two instruction goals remain above 1.005; residual attribution
  and final correctness rerun pending. Existing T057 search coordinate and
  half-surrogate differences remain outside this fix.
- Next action: resolve the remaining instruction costs, rerun correctness
  gates and hand off under the unchanged lead decision below.

## Goal and scope

Enforce the current UTF-8 storage invariant from the
[String design](../docs/design/string-code-units.md), without changing primary
storage, environment bindings or consumer key algorithms. Same code units must
have the same bytes before reaching property storage or a string search.
Base: `cb8ec3ad`; owner branch: `agent/string-canonical/codex`.

## Implementation

`qjs_unicode::wtf16::push_js_str` checks only the two seam scalars. Its ASCII
suffix path reads one byte before Rust's ordinary append; merging rewrites at
most eight seam bytes. `push_js_char` lets lexer and runtime code-unit builders
use the same rule. The shared Unicode crate already serves both crates, so no
new dependency is needed. `JsString::new` validates only in debug builds.

Migrated producers: lexer string/template escapes and runtime-source sentinels;
primitive/general `+`, local/global/property `+=`, compiler-known literal
concatenation, template assembly; `String.raw`, `concat`, `fromCharCode`,
`fromCodePoint`, code-unit slices; array join; repeat and padding; JSON parse
escapes and nested stringify indentation; case conversion; literal and RegExp
replacement output/substitutions; RegExp escaping; global unescape. Normalize
and toWellFormed preserve canonical input, including actual sentinel-range
scalars. Atom interning retains defensive canonicalization: its untyped `&str`
API accepts raw text independently of `JsString`, so removing it is not part
of this fix. No consumer adds a normalization scan. Padding streams, counts
and truncates UTF-16 units without materializing the whole pattern; repeat
checks its maximum length in units before allocating.

## Coverage

- `string/canonical/tests.rs`: seam combinations, every U+F0000..F07FF scalar,
  repeat/pad seams, alias preservation, debug assertion, handle equality/hash/order;
  odd-unit padding truncation, receiver length/coercion and repeat overflow.
- `value/object/tests.rs`: verifies all four actual storage forms and both
  stored/query spellings, with only one property per code-unit sequence.
- `string-canonical-keys.js`: 28 producers crossed with property operations,
  literal/dynamic/array/function/class/fromEntries/JSON receivers, reverse
  spelling combinations, Symbol registry, global/eval/with binding lookup,
  Proxy invariants, whole-string search/split/replace and localeCompare.
  Includes JSON indentation seams and real sentinel-range round trips.
- Lexer coverage pins canonical literal/template bytes and source spans.
  Old matcher fixtures now build valid canonical subjects; Unicode matcher
  internal expectations use stored-scalar coordinates, as its API requires.

## Verification

Initial correctness commands, first padding failure and completed 5107-case
scoped coverage are recorded in [the correctness verification archive](archive/T063-correctness-verification.md).
The lead decision below controls acceptance; instruction diagnostics do not
replace its wall-time screen.

## Instruction regression follow-up (2026-10-20)

Lead feedback authorizes instruction-only diagnosis and a general repair under
this correctness task; it does not open a performance promotion or change the
lead decision below. Baseline `cb8ec3ad`, reviewed candidate `41f4ee96`.

Before further runtime edits, exact-image no-timing job
`20261020T092510Z-t063-instruction-diagnosis-r2` (exit 0) reproduced the
regression. `property_dynamic_read` has three existing string keys, no loop
concatenation. N=2M/4M, three alternating pairs: 618.7032 versus 636.0159
instructions/property, +17.3127. Both five-second exact-binary samples execute
the same wide activation/property-read chain; the untouched wide executor
body grows from 24736 to 26704 bytes with additional spill/reload operations.
The ordinary VM activation assembly normalizes identically; release binaries
contain no canonical-check assertion or validation routine. Other case ratios
and final evidence will be recorded after the repair.

Diagnostic repair to test: keep the ASCII append wrapper small and inline,
and isolate UTF-8 seam decoding/reconciliation in one cold out-of-line helper.
This retains the canonical invariant and constant seam work, and removes the
large rare branch from callers' inline graph. Confirm with the exact x86 build
recipe and the same nine cases, then rerun correctness gates. No benchmark
recognition, selection acceptance or timing job is authorized to this owner.

The first cold-seam repair `cc68ea0d` preserves all nine focused tests, but
no-timing job `20261020T092901Z-t063-cold-seam-instructions` (exit 0) does not
meet the instruction goal: property_dynamic_read 1.01498, raytrace 1.01009,
cdjs 1.01183, access-binary-trees 1.01930, base64 1.01272, lodash 1.01111,
tagcloud 1.01484, unpack-code 1.00091, js-yaml 1.00782. Wide activation is
25856 bytes, still above the baseline. These are diagnostic counts, no wall
result or acceptance. Next isolate the whole canonical append, including its
allocation/copy tail, behind a non-inlined shared call rather than importing
that machinery into runtime callers. Keep the cold seam helper and tests.

Whole-append outlining `5a8147c4` also misses the goal: no-timing job
`20261020T093457Z-t063-shared-append-instructions` completes all nine case
counts, but exits 1 before its extra probes because the synthetic append probe
incorrectly calls unavailable `console`. No sentinel or per-append conclusion
comes from that failed tail. Property ratio 1.01493, wide body still 25856
bytes; outlining the allocation tail did not remove the remaining pressure.
Restore the small inline wrapper; isolate the wide driver's general operand
ownership/clone helper instead. It is used only after the numeric fast path,
so numeric register arithmetic stays inline. This directly limits the large
Value-clone branch imported into the hot driver; it changes no ownership rule
or string invariant. Verify forwarded locals and consumed temporaries, then
repeat the original exact-recipe instruction cohort and corrected probes.

The operand-call boundary `073cc2d1` also misses the goal (property ratio
1.02030); a smaller function alone is insufficient. Revert that boundary.
Disassembly shows register stores materialize the old payload on the stack
before checking whether it owns a resource. Next test the discriminant first:
non-owning slots publish the new value without keeping an old payload alive;
a separate non-inlined owned-store helper preserves publish-before-drop for
the remaining variants. This bounds the actual store lifetime, rather than
adding calls to hot reads. Add a refcount/replacement regression test.

Store-lifetime repair `623cf4d2` brings seven of nine cases below 1.005;
remaining ratios are access-binary-trees 1.00783 and tagcloud 1.01453.
The lexer scalar producer already excludes the entire sentinel range before
its ordinary branch, so that branch cannot need seam repair. Restore its
direct scalar append instead of redundantly calling the sentinel-aware helper.

The scalar-producer fix reduces tagcloud only partially. Residual short exact
samples (`20261020T095916Z-t063-residual-short-samples`, exit 0) are too sparse
for percentages, but show the new owned-store call in the hot path. Its x86
body adds a second out-of-line `drop_in_place<Value>` plus an old-enum stack
copy. Expand that bounded enum dispatch inside the existing non-inlined owned
helper, preserving publication order and each field's normal Rust drop.

Tagcloud phase probe `20261020T100511Z-t063-tagcloud-phase-instructions`
(exit 0, five alternating pairs) isolates setup at 1.01254; the makeTagCloud
body's instruction difference is 0.99756. The residual is not its append loop.
Code-unit encoding still sends ordinary BMP units through the sentinel-aware
scalar helper. Only low surrogate units can form a seam pair: send those to
the shared helper, append all other units directly, and retain every test.

### Lead regression-screen decision (2026-10-20, before any formal result)

Fixed before the formal run; quick triage `20261020T084226Z-t063-quick-triage`
(base `cb8ec3ad`, candidate `41f4ee96`) was already queued as a diagnostic early
stop and cannot pass the change. Instead of the full v6 acceptance chain
(~4.2 h, sized for claimed wins), this correctness fix gets one 30-block
workers-4 three-engine `compare`, base `cb8ec3ad`, candidate `41f4ee96`
(runtime identical to `8e5a2c3f`). It passes when the job is not DISTURBED,
each lane geomean candidate/base is at most 1.010, and no case has a 95%
interval lower bound above 1.030. A case or lane that fails gets a no-timing
instruction count against the base: instruction ratio at most 1.010 marks a
placement effect and does not block; above it, the extra work is fixed and the
compare is rerun. The quick-triage outcome only decides whether the compare is
worth queuing; neither the thresholds nor this rule change after a result.
