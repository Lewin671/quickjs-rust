# T057: String code-unit migration

- Status: active; stage 1 promoted/integrated; stage-2 performance unit abandoned.
- Implementation: stage 1 present on main; minimal correctness extraction present
  in this branch; stage-2 view/cache implementation `6000a9c1` is not imported.
- Verified at: base `ae45226e`, 2026-10-20; four requested differences reproduced;
  candidate tree: fast gate (2329 runtime tests), 255 NG fixtures and
  instruction diagnostics pass; full gate/final String scan pending below.
- Evidence: [stage-1 retained evidence](archive/T057-stage1-evidence.md);
  committed unit fixtures/tests; stage-2 closure and correctness record below.
- Unresolved: full gate/final String scan and lead's regression-screen decision.
- Next action: complete correctness gates and instruction-only diagnostics;
  lead freezes the proposed screen before any timing measurement.

## Goal and scope

Follow the [code-unit authority design](../docs/design/string-code-units.md).
This extraction fixes UTF-16 coordinates and half-pair matches locally in
String search, literal split and replacement helpers. No UnitView, cache,
primary storage, interpreter, parser, dependency or third-party change.
ASCII retains byte search/copy and existing allocation paths. Non-ASCII
operations use temporary per-operation unit buffers or streaming comparisons.

Owner: `agent/t057-correctness/codex`, base `ae45226e`; worktree
`worktrees/quickjs-rust/t057-correctness`. No merge, push or worktree removal.
[Earlier coordination](archive/T057-log.md) and
[stage-1 promotion](archive/T057-stage1-evidence.md) are historical evidence.

## Stage-2 performance closure

Frozen plan `7d3b26c1`, `tasks/performance-units/string-unit-views-primitives.json`,
base `a68a9f46`: **abandoned after diagnostic triage; 0 acceptance attempts;
no promotion**. Runtime candidate `6000a9c1` remains on its original branch.
Plan SHA-256 `0c369e17ad560bd859d618e975c63f173dcae95072c84aeacb9525faeacb80f3`.
The JSON is referenced at its original commit and remains byte-identical;
repository rules require a committed immutable plan, not its import into main.
No plan merge or JSON copy is needed for this correctness-only extraction.

Triage `20261019T234849Z-t057-stage2-triage` finished exit 0 / 335 s but
is DISTURBED (one heavy `searchpartyuseragent` sample). Its wall result is
inadmissible. Instruction-only findings: broad/string_slice 1.0735 (about
+7.4%), because `slice_units` sent ASCII through UnitView; recursive_call_tree
1.0495, from the numeric-helper argument memcpy code-generation artifact
subsequently repaired by [T058](archive/T058-instruction-diagnosis.md).
property_dynamic_read was 1.0150. These are diagnostics, no acceptance result.

Selection `20261020T001249Z-t057-stage2-selection-k6` stopped at its prerequisite
(exit 1, 0 s), explicitly recording zero spent attempts. Dependent jobs
`20261020T001250Z-t057-stage2-{formal30,formal-power}`,
`20261020T001251Z-t057-stage2-heldout-k6` and
`20261020T001252Z-t057-stage2-decide` were cancelled/stopped before measuring
(exit 2 on prerequisites); receipts job `20261020T001252Z-t057-stage2-receipts`
exited 0. No formal/panel/held-out result exists for this unit.

## Correctness extraction

Base `ae45226e` (includes T063) vs pinned NG:

| Probe | Base | NG |
| --- | --- | --- |
| `'😀x'.indexOf('x')` | 1 | 2 |
| `'😀x'.startsWith('x', 2)` | false | true |
| `'😀'.includes('\uDE00')` | false | true |
| `'😀'.split('')` first-unit codes | [55357] | [55357,56832] |
| `'😀x😀'.lastIndexOf('x')` | 1 | 2 |
| `'😀'.lastIndexOf('\uDE00')` | -1 | 1 |
| `'😀x'.endsWith('\uD83D', 1)` | false | true |
| `'😀x'.replace('x', (m,p) => p)` | 😀1 | 😀2 |
| `'😀'.replaceAll('', '|').length` | 4 | 5 |
| `'😀'.split('\uDE00')` part lengths | [2] | [1,0] |

String-pattern search/match already return unit indices (2 for x, 1 for
low-half search); match returns the lone low unit. `at(1)` is 56832;
iterator over pair/high/x/low returns lengths [2,1,1,1]. These agree on
base and NG and remain unchanged, pinned by the regression fixture.

Current candidate: fast gate `check-touched.sh --staged --explain` exit 0,
2329 runtime tests; no automatic Test262 filter matches String source paths,
so exhaustive String and StringIteratorPrototype slices are run explicitly.
`compare-qjs.sh` exit 0, 255/255 fixtures (three new family fixtures).
The three eval-level unit tests share these fixtures; the low-level helper
unit test independently pins forward/reverse and interior-pair boundaries.
Full gate and final String scan results will be finalized before handoff.

## Instruction-only diagnosis

Local arm64 macOS 26.6.2, rustc 1.95.0; identical release recipe (fat LTO,
16 codegen units, abort, default features; agents off), `/usr/bin/time -l`.
Three alternating base/candidate pairs; warm each binary once; equal stdout
hashes in every pair. Wall time/cycles are discarded. iMac was occupied by
another timing job at build start; no queue or timing job was submitted here.
Raw `target/t057/` files are local artifacts, not retained; conclusions below
are durable. These arm64 counts are not comparable to prior x86 counts.

| Case | Candidate/base retired instructions (median) |
| --- | ---: |
| broad/string_slice | 1.000953 |
| broad/property_dynamic_read | 0.990502 |
| sentinel/recursive_call_tree | 1.000064 |
| SunSpider string-base64 | 1.002913 |
| SunSpider string-fasta | 0.994177 |
| SunSpider string-tagcloud | 0.997786 |
| SunSpider string-unpack-code | 1.005975 |
| SunSpider string-validate-input | 1.008014 |
| development/js-yaml/document | 1.000372 |

Broad uses the 2M-to-4M instruction increment; recursive uses 65536-to-131072
iterations (127 calls each). Other rows are whole generated, hash-verified
workloads. The initial recursive 2000/4000 diagnostic had noisy increments
0.969932..1.018239; it establishes no work change. Increasing its length gives
increment ratios 0.997495, 1.000279, 1.000064. No favorable wall rerun or
acceptance attempt is involved. All nine instruction medians are below 1.010.

Exact local image SHA-256:
base `6b9adc00f7e8cb4eae37a3efed3e42516029ccbbcf8abe90468d398243a1a589`;
candidate `2955dc69af9e2ad68155e1b78a658f3712ead885744971c3ea094c7416fa7269`.
`compact_fn::wide::activation::run` address-sorted text-symbol span (`nm -n -C`,
repository symbol-size parser): **18768 -> 18384 bytes**, -384. It did not
grow; sensitive property/recursive instruction controls show no excess.
No x86 function-size or spill comparison is claimed.

## Proposed regression screen (lead must freeze before measuring)

Correctness fix, no claimed win: one undisturbed 30-block workers-4
three-engine comparison against `ae45226e`, exact committed candidate.
Each broad/sentinel/external/development lane geomean must be at most 1.010;
no case's 95% interval lower bound may exceed 1.030. A failed case/lane gets
no-timing base/candidate instruction follow-up: ratio at most 1.010 identifies
a placement effect and does not block; above 1.010 requires fixing the added
work and repeating the comparison. This is a proposal, not an accepted or
executed screen. This owner submits no compare/loop/--timing job.
