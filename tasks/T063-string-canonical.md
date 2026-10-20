# T063: Canonical surrogate-pair string construction

- Status: active; correctness and instruction repair verified; lead screen
  and integration pending.
- Implementation: canonical builders and staged instruction repairs present
  through `d5787373`; no merge or project-remote push.
- Verified at: `d5787373`, 2026-10-20; fast/full gates, 252 NG fixtures, 671
  focused and 5170 complete allowlist cases; nine instruction goals pass.
- Evidence: [initial correctness](archive/T063-correctness-verification.md) and
  [instruction repair](archive/T063-instruction-diagnosis.md) records; raw logs
  are local workspace artifacts, not retained in this repository.
- Unresolved: lead wall-time screen pending; three unconfigured upstream
  cases fail identically on base and repair. Existing T057 search coordinate
  and half-surrogate differences remain outside this fix.
- Next action: lead reviews the repaired branch and runs its unchanged
  acceptance rule before integration; retain the owner worktree.

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
Instruction repair keeps UTF-8 seam reconciliation and surrogate-unit emission
out of ordinary inline paths. Register stores test the old tag before moving
payloads and isolate owned field drops with the same publication order.

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

## Instruction regression follow-up

The repair at `d5787373` meets the requested instruction goal on all nine
cases (highest median 1.001043); all six sentinels stay within 0.999957..1.000797.
[The instruction diagnosis record](archive/T063-instruction-diagnosis.md) owns
exact images, root-cause evidence, all failed experiments, per-case ratios,
per-operation counts and rerun results. This is instruction-only evidence;
there is no new wall-time acceptance or change to the lead decision below.

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

Candidate amendment (2026-10-20): the diagnostic triage showed 1.2-2.8%
extra instructions on non-string cases; the cause (wide-executor register
store code generation and a per-unit helper call) was fixed in `d5787373`.
The formal compare above therefore runs against candidate `32e085f8`
(runtime identical to `d5787373`) instead of `41f4ee96`. Base, thresholds
and the failure procedure are unchanged.
