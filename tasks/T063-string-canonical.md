# T063: Canonical surrogate-pair string construction

- Status: active; first full gate exposed padding unit-count regression; fixed
  in working tree, final gates pending.
- Implementation: present at `3504e994` plus padding/repeat length correction
  in the working tree on `agent/string-canonical/codex`.
- Verified at: base `cb8ec3ad`, 2026-10-20 working tree; `cargo test --workspace`
  passes (runtime 2321 tests); staged fast gate passes including 163 Test262
  cases. Full gate and NG comparison pending.
- Evidence: regression fixture and unit tests listed below; preliminary local
  test log is not retained.
- Unresolved: full gate, NG comparison and focused Test262 slices pending;
  performance regression screening belongs to the integration lead.
- Next action: finish gates, commit this isolated fix, then hand off without
  merging or publishing the branch.

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
of this fix. No consumer adds a normalization scan. Padding counts and truncates UTF-16
units; repeat checks its maximum length in units before allocating.

## Coverage

- `string/canonical/tests.rs`: seam combinations, every U+F0000..F07FF scalar,
  repeat/pad seams, alias preservation, debug assertion, handle equality/hash/order.
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

## Verification and merge screening

First iMac no-timing job `20261020T081603Z-string-canonical-gates` at
`3504e994`: full gate exit 1, `String/prototype/padEnd/normal-operation.js`
failed with `Expected SameValue to be true`. All preceding stages passed.
Canonical literals made the old scalar-based filler truncation observable;
padding now streams code units, and repeat's length limit also counts units.
The dependent compare/slice commands did not run after that failure.

Pending final commands:

```sh
./scripts/check-touched.sh --staged --explain
./scripts/compare-qjs.sh
./scripts/test262-subset.sh --filter test/built-ins/String --filter test/built-ins/Object --filter test/built-ins/JSON --filter test/built-ins/Proxy
./scripts/check.sh
```

The integration lead should screen the exact candidate against `cb8ec3ad`
using the current qualified performance policy: complete selection controls
and lane ceilings, required layout persistence, and held-out screening. Watch
string-append/concat, join/repeat/padding, string-validate-input, papaparse,
regexp-dna and js-yaml as diagnostics, without selecting programs to shape a
correctness special case. Keep instruction counts alongside wall time to
separate helper work from binary placement. No timing jobs are submitted by
this owner. This change adds constant seam work on appends and code-unit
emission; repeat retains Rust's bulk path when its seam cannot merge. Debug
construction alone adds a full validation scan. Existing non-ASCII search
coordinate and half-surrogate search differences remain later T057 work.
