# T056 stage A: outlining numeric property misses

- Status: diagnostic follow-up to the incomplete placement probe; final wall
  regression and complete new-image correctness remain unverified.
- Implementation: stage-A branch only; shared numeric shape hits stay inline,
  layout revalidation and boxed typed-property fallbacks are outlined.
- Verified at: b904285a + final patch 201ecc13 / image 66ffb46f: local full gate,
  255 NG fixtures, strict reference pin and 86/86 instruction diagnosis pass.
- Evidence: [placement outcome](T056-m1-stageA-placement-diagnosis.md#probe-a-returned-incomplete-move-to-code-generation),
  [earlier numeric repair](T056-m1-stageA-attempt1-diagnosis.md#nbody-follow-up-numeric-calls-and-scalar-spills).
- Unresolved: fresh committed-image exact/pre-gate/triage receipts; wall effect.
  The previous 42,672-pass exact receipt belongs only to image 807867f6.
- Next action: lead runs the new bound verification packet in sequence. Do not
  submit the remaining K=6 attempt until diagnostics justify it.

## What the old hot paths actually do

Both canonical images are the frozen base f58c696e / b0dfc65d and repair
4842f823 / 807867f6. Local_read's traced operations are Lt, Exit, Add, Add,
Update; branch_arithmetic adds BitwiseAnd, StrictEq, JumpIfFalsy and a
forward Jump. All traced scalar operation counts match across implementations.

Static selected numeric paths (from opcode entry through the jump back to
main dispatch, excluding the common dispatch header) are:

| Body / numeric operands | Add | StrictEq | BitwiseAnd | Lt | Update | true Exit | true JumpIfFalsy | forward Jump |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| base WideLoopFrame | 32 | 34 | 59 | 32 | 17 | 12 | 10 | 3 |
| old WideLoopFrame | 32 | 34 | 59 | 32 | 17 | 12 | 10 | 3 |
| base Vm | 32 | 34 | 59 | 32 | 17 | 12 | 10 | 3 |
| old Vm | 34 | 36 | 62 | 33 | 17 | 12 | 10 | 3 |

Both Wide Binary paths restore **one loop-invariant non-scalar pointer** from
rbp (base -0x30 scratch owner; old candidate -0x38 boxed bank). Neither
spills/reloads the scalar-bank pointer there. Update, Exit, JumpIfFalsy and
forward Jump contain no rsp/rbp memory operand. The dispatch header and
residency backedge keep pc/iteration state in registers. Binary result stores
write the scalar register bank, as required by this tier, rather than a new
stack-carried intermediate. Consequently the 376→680-byte reservation is
**not evidence of a new local_read/branch_arithmetic stack dependency**.
This static audit does not establish why their equal-count wall time differs.

The old Vm Add arm additionally saves/restores the live rsi value in r13
(two register copies); the BitwiseAnd path has one further copy. This is a
concrete code-generation instruction increase, without a source semantic
change. It is separate from the equal-count Wide regression.

These are finite small numeric operands and true-boolean control paths,
not all numeric edge cases or a dynamic hardware trace. Tables were read
from the exact Mach-O dispatch data; code decoding stops before embedded
jump-table data. The original linear otool counts include data interpreted
as instructions and must not be used as executed instruction counts.

## General source boundaries

The new layout's Small numeric cache was expanding metadata validation,
generic Pair replacement and owned Value temporaries into dispatch.

- Keep shared Small shape identity, namespace, borrow, bounds and numeric
  payload checks inline; keep numeric overwrites inline too.
- On a shape miss, release the RefCell borrow and reborrow in a cold,
  non-inlined reader/writer. No callback or user code runs between those
  borrows, so the single-threaded object's state cannot change there.
- Small layouts have descriptor Slots, as the storage module invariant
  requires. Numeric writers use those slots directly; Pair replacement is
  outside this admitted encoding. Foreign shapes still check key identity,
  data-property kind and writability; Dynamic/dictionary admission is unchanged.
- Replace an existing number's f64 in place; replace an owned non-number in
  an outlined helper that runs the same Value destructor.
- Outline complete GetNamedTyped/SetNamedTyped misses. They retain the
  original object/non-object routing and return Typed/bool, so owned Value
  conversion/drop temporaries do not belong to the main executor.

No dependency, unsafe code, opcode admission, tier reach, source/benchmark
recognizer, binding snapshot or frozen-plan change is involved. The focused
object test checks that a numeric overwrite releases an old ObjectRef, that
an appended shape can revalidate its old slot, and that read-only/reused
slots reject writes. Existing typed-loop tests cover getters, setters,
frozen fields, reordered objects and dynamic storage.

## Non-timing development experiments

All snapshots start from b904285a plus their sealed source patch; each job
records the full patch SHA and image SHA. None is timing/acceptance evidence.
The canonical producer/recipe and frozen base are unchanged. No offsets were
searched; final reference pinning is mechanical and happens after source work.

| Job suffix (20261021T...) | Change | image SHA prefix | Wide stack bytes | local instructions | nbody instructions | unpack instructions |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| 021250Z-t056-codegen-cold-instructions | outline field validation and old-value drop | e685f4b7 | 696 | 1.00007 | 0.99415 | 1.01246 |
| 021505Z-t056-codegen-owner-instructions | original scratch-owner parameter | c5a17736 | 632 | 0.96967 | 0.99265 | 1.01221 |
| 021649Z-t056-codegen-closure-instructions | force execute inline, without closure barrier | bae5bb49 | 520, combined run | 1.09657 | 1.07031 | 1.01343 |
| 022106Z-t056-codegen-context-instructions | three borrows behind one mutable context | 36703ad3 | 632 | 1.00011 | 0.99562 | 1.01366 |
| 022344Z-t056-codegen-small-payload-instructions | whole layout miss outlined; Small-only payload | 82bcbed9 | 616 | 1.00004 | 1.00089 | 1.00172 |
| 022604Z-t056-codegen-typed-fallback-instructions | typed property fallbacks outlined | d1d6df8d | 600 | 1.00002 | 1.00735 | 1.00162 |
| 022912Z-t056-codegen-entry-boundary-instructions | explicit closure boundary; original object miss routing | d15798cd | 360 | 1.00007 | 1.00858 | 1.00106 |

The forced-inline-only variant includes seeding/teardown in run and clearly
adds instructions; it is rejected. The context bundle does not restore the
old frame. Source boundaries must be judged on actual linked bodies, not
source annotations alone. The final variant adds an explicit non-inlined
entry closure, with execute forced into that closure (the baseline's actual
boundary), so seeding/teardown stay outside numeric dispatch.

## Other instruction increases

Exact base/4842f823 attribution job
`20261021T021538Z-t056-codegen-increase-profiles` completed without changing
production images. The external programs were repeated 40 times for CPU
sampling only; sample counts are attribution, not instruction or wall ratios.

- Bitwise-and: 282/283 base and 292/293 candidate self samples are in the
  **Vm** typed executor. The old Vm's extra numeric-arm copies above provide
  a general fixable cause; the Wide-only placement explanation does not apply.
- Class-fields: ordinary wide activation has 3139/6656 base and 3014/6661
  candidate self samples; cloning and ConstantFieldSegments::install also
  appear. Its ordinary wide stack reservation is unchanged at 0x388. This
  profile does not establish an extra executed spill or a metadata-work count.
- Unpack: ordinary wide activation has 234/3209 base and 263/3242 candidate
  self samples; allocation/freeing and regexp/string processing dominate.
  The Small-payload outline reduces its instruction excess from about 1.3%
  to about 0.17% in a sealed diagnostic. That is a general code-generation
  effect; the samples do not prove allocator/env state caused the old excess.

The final complete instruction inventory must check all three explicitly;
no isolated profile percentage is an acceptance result. Probe B remains
unsubmitted; its original completion guard is not relaxed.


## Final dispatch boundary before reference re-pin

Snapshot `20261021T022912Z-t056-codegen-entry-boundary-instructions` is
b904285a + patch SHA-256
`7c58b2064c617ccec917a2195c58d0bd2f5f56e496aad5579031f8cec695ac4f`;
image SHA-256 `d15798cdfbc93aa2689bea167c69036ec799604fc863fa59362a4c0565a9616e`.
Both actual dispatch bodies are run-entry closures. Their stack reservation
is **0x168 = 360 bytes**, below base's 0x178 = 376 and old candidate's
0x2a8 = 680. The inner execute is forced inline; the entry closure is forced
out of line. Preparation, seeding and teardown remain in the outer run.

The selected Wide and Vm numeric paths now both match the base's first row
in the table above, including exactly one invariant-owner load per Binary
and no stack operand on Update/Exit/JumpIfFalsy/forward Jump. No new
loop-carried stack dependency exists on those audited paths. Outlined
`shared_data_slot_{number,write_number}_after_miss`, `replace_with_number`,
`get_named_typed` and `set_named_typed` own the cold graphs. This establishes
a smaller frame and a stable register-allocation boundary; it does **not**
establish that the unexplained old Wide wall loss has been removed.

At this snapshot global instructions are 0.999996, dynamic-property 0.925172,
trees 0.986541; moment/diff/meriyah are 0.965313/0.967014/0.967263. Existing
instruction gains survive the structural change. Whole-portfolio instructions,
reference residues, committed-image correctness and wall triage are still
required after re-pinning; these ten-case diagnostics cannot substitute for them.


## Reference-pinned image

Non-timing job `20261021T023140Z-t056-codegen-pinned86-instructions` builds
b904285a + patch SHA-256
`201ecc13de4b746938ad0339dc9f3799b652180d95cab9a10bd322c14101fe31`.
Canonical image SHA-256:
`66ffb46f31c226076b6b4fde1405c84dcef789ed382613d8c67a2f10bdd7136e`.
Reference-pinned order SHA-256:
`6d49aaca8f7d9d03c901779f7a385dbf55b1dae6273e9d3cd7a2e09e54d94c20`.
Strict audit passes; both executor page residues (Wide 0xd90, Vm 0x110)
and all 15 surviving reference callee residues equal base's. There was no
offset search. The final Wide closure is 0x100004d90, extent 13696; Vm is
0x100001110, extent 13792. Their audited frames remain 360 bytes after link.

After replacing branch addresses and RIP displacements by labels, all eight
selected paths above have **the same operations and register operands as
base** in both bodies, not merely equal counts. The false-boolean
JumpIfFalsy path also matches (16 instructions, zero stack operands).
This comparison does not assert equal branch target placement, identical
relocations, every numeric edge path or wall-time equivalence.


## Final checks and lead handoff

`20261021T023140Z-t056-codegen-pinned86-instructions` completed exit 0 in
258 s, no DISTURBED. All 86 policy IDs occur exactly once with one instruction
pair; every ratio passes its original max(1.03, instruction_band) admission
threshold. This is an owner instruction diagnosis, **not** a bound replacement
for lead's new correctness→pre-gate chain, and no wall conclusion is drawn.
The instruction report SHA-256 is `b578429df6a9e55dc1e9d25a7c09485d9de7e86710dd6ed5b349c3deb1b049f0`.

| Case | Final/base instructions |
| --- | ---: |
| broad/local_read | 0.999966299 |
| broad/branch_arithmetic | 0.999990767 |
| broad/global_read | 0.999968334 |
| broad/property_dynamic_read | 0.925104181 |
| external/sunspider-1.0/access-nbody | 1.008851039 |
| external/sunspider-1.0/access-binary-trees | 0.986493777 |
| external/sunspider-1.0/string-unpack-code | 1.001630768 |
| external/sunspider-1.0/bitops-bitwise-and | 1.009699325 |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.016045869 |
| development/moment/dates | 0.965475496 |
| development/diff/compare | 0.966883564 |
| development/meriyah/parse | 0.967529440 |

The largest increase is math-spectral-norm 1.018325 (then graphlib 1.017194,
class-fields 1.016046). Bitwise-and improves from 1.025654 to 1.009699,
unpack from 1.013646 to 1.001631. **Residual bitwise and class-fields work is
not eliminated**; the selected Vm numeric paths are repaired, but these
whole-program residuals are not attributed to a proven extra hot spill.
Avoid extending this change into unrelated allocation/constructor policy
without new attribution. All remain visible in the fresh lead triage.

The final source passed local `./scripts/check.sh`: workspace runtime 2346,
agents 21, diagnostics 29, benchmark-tool tests 663, script tests 35 and
Test262 subset 5170. `./scripts/compare-qjs.sh` passed 255 fixtures; 26 focused
typed-loop tests and the new object ownership/attribute test also passed.
Context, formatting and whitespace checks pass. Complete new-image exact
42,672-case correctness is deliberately prepared for the lead, not claimed
from the old image; no new burndown entry is appended before it completes.

Lead packet: `tmp/t056-m1a-codegen-verify/README.md` under the shared workspace.
It binds the committed source, exact image SHA above, unchanged unit d184dbcc,
and a helper SHA; old 484/944/0a receipts cannot satisfy it. It prepares:

1. Non-timing full checks/NG/subset and all eight disjoint exact shards,
   verifying the entire pinned corpus and zero-gap result.
2. Non-timing 86-case instruction pre-gate, requiring that new correctness
   receipt and the same image hash.
3. Lead-only T061 natural-layout diagnostic, all 19 fixed controls plus
   12 declared focuses (27 unique, including class-fields), five pairs and
   four workers, requiring both receipts. Its native budget is 30 minutes.

The copied ZoneInfo night-window guard and completed-job/no-DISTURBED checks
remain intact. Renderers only print commands. No timing job, probe B, final
K=6 attempt, main integration or main push was submitted by the owner.
Valid attempts remain one spent and one remaining across A+B.
