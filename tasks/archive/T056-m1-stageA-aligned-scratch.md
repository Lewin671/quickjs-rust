# T056 stage A: aligned scalar scratch and literal numeric reads

- Status: production follow-up implemented on the stage-A branch; local verification
  complete, no new acceptance attempt or wall result.
- Implementation: safe 4096-aligned scalar backing, existing program-local pool;
  literal numeric cache hits copy f64 directly, with the full fallback retained.
- Verified at: 4e609638 + source patch3c8ceddb / canonical b0b84489; local full
  gate, NG255, strict pin and86/86 pass. Old d130 exact does not cover this image.
- Evidence: [data diagnosis](T056-m1-stageA-data-diagnosis.md), completed
  job20261021T040650Z and the source/image identities below.
- Unresolved: new image exact correctness, bound pre-gate and diagnostic wall
  triage remain lead work; unpack has no demonstrated causal fix.
- Next action: lead runs the new correctness -> bound86 -> diagnostic triage
  packet. Attempt1/2 remains spent; one remains across A+B, no automatic K=6.

## Completed data probe

`20261021T040650Z-t056-m1a-data-probe` is done0/no DISTURBED, 401.024s:
three cases,15 complete blocks,16/10/10 roles,540/540 units, all outputs match.
Effects SHA-256:
`dac7a85d3fab556d6b3adddaea6afa65bada90b77df2a6ca92e9fed6a12acbd6`.
These are exploratory, unadjusted95% intervals, not stage acceptance.

| local_read comparison | wall ratio [interval] |
| --- | ---: |
| canonical C/B natural | 1.070371 [1.056174,1.097463] |
| canonical C/B MallocNanoZone=0 | 1.005115 [0.994823,1.016326] |
| canonical C/B env128 | 1.064657 [1.049425,1.091895] |
| base bank00/natural | 1.027142 [1.020362,1.034270] |
| candidate bank00/natural | 0.889033 [0.877331,0.896404] |
| C/B bank00 | 0.918787 [0.912404,0.932188] |
| C/B bank64 | 0.927525 [0.916892,0.930036] |
| base bank64/bank00 | 0.996083 [0.984122,0.999015] |
| candidate bank64/bank00 | 0.994936 [0.986456,1.006686] |

Natural local instructions C/B1.000026; zone0 C/B1.000016. The original
canonical pair's zone0 response strongly supports heap/launch sensitivity.
The64-byte window offset is not material in these diagnostic images. However,
bank00/natural changes code, owner header8->16 and backing4096->8192 as well
as placement; it is a bias control, not pure isolation of scalar-bank address.
Only bank64/bank00 holds those factors fixed within an image. Do not claim
cache-line sharing or4K aliasing as established, or transfer its wall gain to
this new, smaller production wrapper without timing it.

Heterogeneous natural C/B1.043078 [1.032443,1.058542], instructions0.977310;
unpack1.032570 [1.028837,1.042761], instructions1.001586. Zone0/zone1/env128
C/B are respectively1.043909/1.039081/1.032560 for heterogeneous and
1.037147/1.032825/1.033222 for unpack: no comparable cure from these factors.
Natural A/A base1.007232 [0.996258,1.018902], candidate0.990812
[0.726360,1.009315]; retain the wide candidate interval rather than claiming
all launch noise eliminated.

## Production rule and coverage

`typed_loop::RegisterFile` owns `Box<ScalarRegisters>`; `repr(align(4096))`
is on the backing, not the owner or interpreter frame. The backing is4096
bytes, the owner remains one pointer (8 bytes on x86_64), scratch80 bytes.
Safe Box allocation honors alignment; no unsafe, FFI, dependency or global
state. Existing take/recycle pooling allocates only when the pool is empty,
including simultaneous nested entries; reset/clear remains unchanged. Other
scratch vectors stay pooled without new alignment: there is no equivalent
hot-bank evidence and their variable capacities make the cost different.
4096 is the requested alignment, not a claim about every host's OS page size.

The pool test checks alignment, size, same-pointer reuse, distinct aligned
active entries and clearing. Numeric literal-cache tests exercise Pair/Shaped
layouts, changed field values and types, and an accessor transition. The new
`literal_data_slot_number` checks module-namespace exclusion and layout pointer
identity, then copies only Number; layout misses or other values retain the
existing semantic path. Cached layout owns key/attributes and invalidation;
no new cache admission, name snapshot or benchmark recognition is added.

## Heterogeneous executed path and repair

The workload uses three literal layouts with three numeric reads per loop.
Exact b0df/66ff CPU job `20261021T031937Z-t056-data-cpu-profiles` finds
base86/139 self samples in the Wide typed closure,22 in
literal_data_slot_value and19 in get_named_object; d130101/144 in the closure,
19 in get_named_object and13 in get_named_typed. Sampling locates work; these
small counts are not a wall comparison. d130 inlines literal_data_slot_value
into get_named_object, so the change cannot be described as simply adding one
net call on this path.

Boundary-correct disassembly, excluding DATA_IN_CODE, shows d130's adapter
calling get_named_object with a Value out-buffer, reading its tag/payload and
writing the Typed return. This adds boxed transport for numeric literal hits.
The new helper first tests the cached literal layouts and copies f64 straight
into Typed, bypassing owned Value cloning/destruction and the generic getter
on numeric hits. It remains outlined; its cold hint preserves the Small-cache
caller's hot numeric fallthrough. No source/bytecode template is recognized.

| Static body (code rows, not executed counts) | base | d130 |
| --- | ---: | ---: |
| ordinary wide run | 5236, frame0x388 | 4910, frame0x388 |
| own_data_property_read | 576, frame0x68 | 535, frame0x58 |
| get_named_object | 471, frame0x68 | 605, frame0xb8 |
| literal_data_slot_value | 69, outlined | inlined |
| get_named_typed | absent | 49, frame0x18 |
| store_owned | 75 | 75, normalized code identical |

The first numeric-helper variant without cold hint gives hetero0.822399 but
nbody1.018104 instructions. Restoring the existing hint gives0.819853 and
1.012212 in the unpinned11-case diagnostic. These are retained development
observations, not a performance acceptance or evidence of a wall improvement.
Final pinned86 measurements supersede them below.

## Unpack: investigated, no speculative repair

The same exact-image sampling repeats unpack40 times only for attribution:
ordinary wide self samples234/3195 base and266/3233 candidate; malloc/free,
regexp and string operations dominate, not the typed-loop executor. Earlier
job20261021T021538Z has the same direction (234/3209 vs263/3242). The smaller
ordinary-wide body and unchanged frame do not prove a spill; store_owned's
normalized body is identical. Ranked remaining hypotheses:

1. Extra layout/payload dependencies and heap working set/cache traffic:
   ObjectData112->120 and Rc allocation request128->136 are verified. Actual
   malloc bucket size and cache misses are not. Zone insensitivity does not
   eliminate capacity/latency effects.
2. Ordinary-wide caller codegen/dependencies or helper placement beyond the
   two executor/15 callee pin. Whole-frame size alone does not identify an
   executed regression; no causal branch or spill has been isolated.
3. Whole-process startup, cold faults or off-CPU work; repeated sampling
   amortizes launch and cannot price it. This would not by itself explain the
   internal heterogeneous N/2N workload's excess.

No global compiler-flag, store redesign or speculative unpack change is made.

## Final image verification and lead handoff

Non-timing job `20261021T043020Z-t056-aligned-pinned86-instructions` is
done0/no DISTURBED,260s; all86 original policy IDs and thresholds pass.
Source is4e609638742a8c7c24b0d7d164eb1f8e71e4e335 plus crates/tools patch SHA
`3c8ceddbb25853c1e60c4d2d9d1b06f03f0dfcfd7f62416839acb3b2c2a5b9bf`;
canonical image SHA
`b0b84489a3f6fd673b06dc4bdc71c8f176387a231ca19fe1da5ea48ca4ee0fd5`.
| Final pinned case | instructions C/B |
| --- | ---: |
| broad/global_read | 0.999969 |
| broad/local_read | 1.000031 |
| broad/branch_arithmetic | 0.999922 |
| broad/property_dynamic_read | 0.925259 |
| sentinel/heterogeneous_property_read | 0.819928 |
| external/sunspider-1.0/access-nbody | 1.012153 |
| external/sunspider-1.0/access-binary-trees | 0.986304 |
| external/sunspider-1.0/string-unpack-code | 1.003169 |
| external/sunspider-1.0/bitops-bitwise-and | 1.009576 |
| development/moment/dates | 0.966298 |
| development/diff/compare | 0.966870 |
| development/meriyah/parse | 0.967202 |

The largest ratio is spectral-norm1.027396 (cutoff1.03); class-fields1.015947.
These instruction results do not establish wall neutrality.
The canonical producer's explicit host target and original recipe are retained.
Strict audit retains both executor residues0xd90/0x110 and all15 surviving
reference callees. Pin regeneration uses current symbol inventory/body budgets
and reference residues; no timing feedback or offset search.

Local full gate passes: runtime2346, tool663, script35, subset5170; NG255 and
26 focused typed-loop tests pass. The new pool and numeric/accessor assertions
are included. Context/size/whitespace checks pass; staged fast gate is required
before commit. Frozen unit raw SHA stays
`d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284`.

The final-image byte audit reproduces all six local paths against base:
Lt32/140bytes, Exit12/51, Add32/141, Update17/81, Dispatch13/48,
Backedge4/22, exactly equivalent after checked address relocation masking.
Report SHA `b01bea07294a95ac6cc08ed62489e8a1eacc96cd2b162abccd575a99a8ca70e7`.
Both typed frame reservations remain0x168 (360 bytes). This audit uses an
outside copy of the old producer with only expected candidate SHA changed;
the historical versioned d130 audit and probe guards are not weakened.

The lead packet at workspace `tmp/t056-m1a-aligned-verify` binds the new
runtime commit, canonical SHA and helper SHA, preserving the original86-case
policy, full pinned exact corpus and all27 native triage cases.
Correctness must finish done0/no DISTURBED before bound86, then lead-only
triage may run. Old d130 receipts cannot satisfy this chain; no owner timing
or main integration is performed.
