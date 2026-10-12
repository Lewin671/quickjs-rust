# T037: First batch-1 formal decisions

- Status: historical-campaign. Original decision under the pre-amendment rule.
- Implementation: present; neither unit was retained by that rule.
- Verified at: `af4d0d3c`, 2026-10-12, sealed 30-block formal bundle.
- Evidence: exact outputs and numbers below; local raw artifacts not retained.
- Unresolved: superseded by the lead's 2026-10-12 rule and re-decision in [T037](../T037-integration-batch-1.md).
- Next action: none; this record is frozen.

## Formal results at af4d0d3c

The 30-block perf-compare completed successfully under the host timing lock;
then both original promotion commands ran under that same lock. Its shell
returned 1 only because --require-retained rejects an inconclusive decision,
not because measurement failed. All target cases and all other case gates passed;
no wall-time divergence exists. Audio-dft alone is imprecise: cycles/base
0.996205, CI [0.906182, 1.040874], relative half-width 9.9344%, 30 valid blocks.
It is declared by the global plan; class promotion includes it as a portfolio
control. Both exact decide outputs have the same decision and reason:

```json
{"decision":"inconclusive","metric":"cycles","reasons":["external/kraken-1.1/audio-dft: insufficient blocks or wide confidence interval"],"wall_time_divergence":[]}
```

| Unit | Target cycles/base | 95% CI | Formal decision |
| --- | ---: | --- | --- |
| global repin: bitops-bitwise-and | 0.116990 | [0.115555, 0.117865] | inconclusive |
| global repin: top_level_function_call | 0.309658 | [0.309318, 0.310071] | same decision |
| class-field-batch-install: raytrace | 0.947281 | [0.943053, 0.948468] | inconclusive |

Original stdout files: target/batch1-global-numeric-binding-sinks-repin-decide-output.txt
(SHA-256 19e6c93ebdf7bf8eb4c88adae99b266c447de9a2552b19b30f8ad726057a7c6a),
and target/batch1-class-field-batch-install-decide-output.txt
(SHA-256 4af24f1df97b3a67c701f6d50d8d7fba76b6da5014507dd4c8f1de2796e695bb).
The full decision JSON files sit next to the comparison directory, named
batch1-formal-af4d0d3c81aad98721661689cf7224d2c17b9781-<unit>-decision.json.
Bundle/queue paths, hashes, all standing and >3% changes live in T018.

Burndown from CI coverage run 38242164403 names exact candidate af4d0d3c:
53572 total, 10900 NG-skipped, 42672 configured Rust passes, zero fails,
timeouts/not-run/parity gaps; NG passes 42602, fails 70. It was recorded once
in docs/conformance/burndown.jsonl after completeness verification. Artifact
file SHA-256 cf78d4178d426c4bcf9362b293e5dbc786702223f81e1e9123cd850e3c152789.

The original queue candidate executable remains linked from the main cache;
formal base SHA-256 is 6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4;
formal candidate SHA-256 is 6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b,
byte-identical to placement. All samples remain in the sealed bundle, with
no outlier removal or rerun. A five-second monitor observed zero compiler
processes during timing. No full formal run was spent after this outcome.

The new queue binds af4d0d3c (measured engine tip), not the later doc/ledger
recording commit. Profile/freeze the next unit against that exact candidate;
its cached executable and queue identity are preserved. No main update was
made; both implementations remain present but formally unretained.
