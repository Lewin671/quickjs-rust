# T057: Development baseline before policy v3

- Status: historical-campaign; complete preliminary baseline, not qualification.
- Implementation: no runtime change; runtime source 351004f3 equals the brief base.
- Verified at: 351004f386f4c2c1d870467ae6e1e481952a330c, 2026-10-17;
  30-block development preview and current iMac queue replay pass.
- Evidence: conclusions/numbers/hashes below; raw files are workspace artifacts.
- Unresolved: no performance candidate or retention decision was measured.
- Next action: see [development policy qualification](T057-development-policy.md).

## Identity and protocol

Job `20261017T082306Z-t057-stage1-profiles-and-development30`, dispatcher
source 353e9b76, exit 0 / 4781 s. Ten programs, three engines, 30 measured
blocks per program, plus separate capability checks. Every checksum and
paired comparison is complete; all candidate/base intervals are healthy.
The load log has zero rows/heavy samples and no DISTURBED warning.
Whole-process times include startup, parsing, execution and shutdown.
All ratios are informational same-host evidence, not a headline engine score.

Both Rust roles use image
`c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`;
candidate source 351004f3, base source b481894d. Pinned NG is f7830186,
image `919f7235f61412ba69faf6aa05b534869aa7d69d1583b3d7aab3cd5b67cd6f40`.
Host iMac.local, x86_64 macOS 12.6, Python 3.13.16; canonical fat16/abort
agents recipe. No held-out program was profiled or selected.

## Development standing

Seconds are individual-role process medians; Rust/NG and Rust/base are
paired-ratio medians, not ratios recomputed from the displayed rounded values.

| Program | Rust seconds | NG seconds | Rust/NG | Rust/base |
| --- | ---: | ---: | ---: | ---: |
| `lodash/transform` | 1.739935 | 0.629639 | 2.753412 | 0.998199 |
| `meriyah/parse` | 2.515893 | 0.758631 | 3.324086 | 0.996235 |
| `immutable/persistent` | 3.681442 | 0.919597 | 4.010434 | 1.000744 |
| `moment/dates` | 10.328181 | 2.719309 | 3.806572 | 0.997957 |
| `diff/compare` | 6.285609 | 1.756392 | 3.578103 | 1.000223 |
| `papaparse/csv` | 33.368167 | 0.983036 | 33.974375 | 0.998473 |
| `js-yaml/document` | 4.233223 | 0.980256 | 4.320725 | 0.997438 |
| `big.js/arith` | 1.997399 | 0.818999 | 2.437508 | 1.000015 |
| `matter-js/simulate` | 2.810727 | 1.265963 | 2.218285 | 0.999958 |
| `graphlib/paths` | 1.271983 | 0.451123 | 2.813842 | 0.998070 |

Papaparse ranks first at 33.9744x NG; the next case is js-yaml at 4.3207x.
This selects an operation family to investigate, not a permissible library
recognizer. Exact-image profiles are retained under this job's `out/profiles`;
T057 owns the papaparse stack-top interpretation.

## Hashes and timeout derivation

- Development raw SHA-256: `e941f3878d711f16c1684ad7bf2e43af66e5c1108470ea3ff48159fce1e6fc3b`.
- Development report SHA-256: `afdfd04a0bc939fd723566619b9c738a69472a0cd411a864566c071800afcfa4`.
- Preliminary schema-4 queue SHA-256: `02d2c01d2dced455e3fb45b0d9d4436250004fbbef5b556daac480b4c79934d8`.
- Timeout derivation job `20261017T082337Z-t057-development-timeout-reference-fresh`
  completed exit 0 / 8 s, replaying all raw and report bytes before derivation.
- [Development manifest v2](../../benchmarks/development-preview.json), frozen
  in fd5c87c2, SHA-256
  `2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13`.
  It retains all ten NG medians and source hashes; max(60 s, 4 x NG median)
  yields 60 s for every program, equally for all roles. No threshold was
  chosen from a candidate slowdown, and no later qualification changes it.

The preliminary run used manifest v1. Its source/body inventory is unchanged
in v2; the version adds frozen reference-derived hang detection. It does not
substitute for the required new sealed four-lane formal A/A and qualification.
