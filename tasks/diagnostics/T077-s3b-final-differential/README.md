# Final S3b seeded differential qualification

Runtime **e1405670**, baseline **3cc12c6f**. The receiver generator in this commit
uses distinct nested counters and binds index expressions to their nearest loop.
Seeds 0–1999 run for each profile: **6,000 programs / 36,000 engine invocations**.
[Request](request.json) freezes binary, producer and all build-input hashes;
[summary](summary.json) retains the runner's original nonzero verdict. Inputs
remain stable. No panic, crash, timeout, reference-internal failure or disagreement
between default, IC-off, IC-verify and combined binding/IC stress occurs.

| Profile | Programs | MATCH | FIXED | PRE | CHANGED | REGR |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| binding | 2,000 | 602 | 1,280 | 258 | 203 | 13 |
| binding-simple | 2,000 | 597 | 1,290 | 277 | 192 | 18 |
| receiver | 2,000 | 2,000 | 0 | 0 | 0 | 0 |

Tags overlap: different observations in one program may belong to different
classes. All 44 formerly invalid receiver seeds now parse/run, and receiver
SyntaxErrors fall from 44 to **zero**. Binding programs deliberately generate
invalid eval snippets and catch their errors; those inputs are retained.
Five differential-runner unit tests pass. No held-out input is resolved or run.

## Triage and spec-correct differences

The [case index](case-index.jsonl) records every source hash and raw tag set.
The [triage inventory](triage.jsonl) covers all **742** programs carrying PRE,
CHANGED or REGR, retaining every differing observation, base/NG/candidate/Node
values and its explanation. It does not rewrite a raw result or waive a mode.
[Metadata](metadata.json) binds the raw-case hash and independent Node version.
The [Node wrapper](node-reference.cjs) evaluates a script and exits immediately
on uncaught top-level failure, like the two CLIs; it does not drain pending jobs
after catching a script failure in its host. This policy matters for order checks.

Every candidate-only non-log observation agrees with Node except **one indexed
stderr entry**, binding-simple/170. Its complete stdout and failure exit match NG;
`value is not callable` lacks the TypeError prefix. Base has the same diagnostic
at an earlier output index; repaired eval observations move its index from 32
to 36. This is an inherited CLI error-spelling limitation, not a binding result.
The same inherited issue affects strict-delete SyntaxError spelling in PRE rows.
It remains visible in the raw verdict and is not normalized away.

The remaining differences have these language rules as their authority:

- Object-record HasBinding and GetBindingValue each call HasProperty; Assign
  also checks presence in SetMutableBinding. NG/V8 often collapse these calls
  or change lookup-probe order. The pinned Test262
  `language/statements/with/get-binding-value-idref-with-proxy-env.js` and sibling
  call/compound cases assert the protocol. All **189** candidate-only log rows
  have exactly Node's **Get/Set event sequence**; 183 differ from Node only in
  HasProperty probes. Compiler-generated names visible in base's logs are gone.
- Catch parameter initialization is part of the try statement's catch evaluation.
  If destructuring fails, finally still executes; NG skips it. Its missing reads,
  writes, global type changes, printed observations and later closure/job values
  account for downstream differences (for example binding/1251's number versus
  boolean). Candidate/Node execute finally. [Small proofs](reference-proofs.json).
- Lexical references retain the selected scope/cell through with, eval, catch,
  classes and suspension. An eval function declaration's variable binding must
  not replace a catch parameter's independent cell. Candidate/Node preserve it;
  NG changes it (small proof: number versus function). Root lexical homonyms and
  escaping closures remain live, including generated binding/394, /1816, /1822
  and binding-simple/413, /485, /949, /988, /1737, /1822 observations.
- Assignment evaluates/GetValues its RHS before PutValue checks an immutable
  target. In binding/806 and binding-simple/806, reading absent `b` must throw
  ReferenceError before writing captured const `y`; candidate/Node do so, while
  NG reports TypeError. The exact reduced program is retained below.

[Reduced reference cases](reduced/metadata.json) retain the nontrivial seed
reductions and exact observation values. Each reduction preserves candidate =
Node and candidate != NG; no runtime code is changed to imitate reference bugs.
All FIXED observations agree with NG and are the binding/effect/Annex-B repairs
recorded in the [round-3 log](../T077-s3b-round3-fuzz/README.md) and its linked
records. No unexplained candidate-only semantic disagreement remains. The raw
runner still exits **1** because its conservative CHANGED/REGR tags deliberately
require this review; this receipt does not call it an all-engine match.

Reproduce using `docs/binding-differential-fuzz.md`, count=2000, jobs=4, profiles
binding/binding-simple/receiver, with the binaries/producers in request.json and
verified [base source](../T077-s3b-round3-fuzz/base-source.json). Final full/exact,
stress/depth/counters/public/order/timing qualification is the next step. Lead
alone owns formal/held-out/RSS acceptance.
