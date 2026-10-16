# T050: Ordinary VM layout causality diagnosis

- Status: closed-landed (diagnostic record only).
- Implementation: none; diagnostic binaries only, no production change planned.
- Verified at: runtime inputs identical to 84ceebc4; full gate at a648dc19,
  2026-10-16; exact-binary and controlled-layout measurements below.
- Evidence: the frozen protocols, binary identities and measured results below;
  raw bundles are supplementary workspace artifacts.
- Unresolved: global best placement and specific microarchitectural cause;
  no performance promotion or production runtime change was attempted.
- Next action: a separately frozen layout-maintenance unit should match current
  symbols and preserve audited hot-function addresses before further runtime work.

## Goal and boundary

This is a causal diagnosis, not a reopening or promotion screen of the exhausted
T049 unit. Runtime inputs are the restored main and the rejected c1c6d13b
property reader, copied exactly. Production runtime, flags and order files
remain unchanged. Owner branch: agent/vm-layout-causality/root, base above;
worktree: worktrees/quickjs-rust/vm-layout-causality in the shared workspace.

## Frozen diagnostic design

Use a 2 by 2 factorial: original/rejected reader, with ordinary VM page offsets
0x020/0x270. A temporary assembly object supplies unexecuted NOP padding before
and after the VM in a constant-size slot. Order all named functions in the same
order and compensate the three previously observed size differences with
padding. Audit actual addresses of every shared text symbol before timing;
report any additional displacement instead of calling an imperfect layout
fully isolated. Runtime source edits are limited to replaying T049.

Measure 15 balanced blocks, rotating all four binaries, with equal-length paths
and identical argv/environment. Include byte-identical A/A copies as a noise
control. Retain absolute raw N and 2N samples, stdout and instruction counts.
Use the existing screen calibration and validation, wall time for inference.
Cases fixed before building: broad branch_arithmetic, global_read, local_read,
property_dynamic_read, property_read, dynamic_method_call; external hash-map,
CDjs, 3d-raytrace; all six sentinel cases. No outliers removed. No retuning of
offsets and no second favorable-run search. Interruption or host DISTURBED
requires a replacement run, preserving the invalid result.

Placement is supported as a cause if its within-reader effect exceeds the A/A
spread, repeats in both reader implementations with the same sign, and the
reader effect at fixed placement is materially smaller than the original
T049 regression. Otherwise report remaining confounding or lack of evidence.
This does not demonstrate a globally optimal offset or authorize retention.

## Verification

Audit source diffs, binary identities, symbol sizes/addresses and workload
checksums. Run repository context checks for the task record. No runtime
candidate will be integrated by this diagnostic task.

## Diagnostic follow-up frozen after phase 1

The factorial run 20261016T113550Z-vm-layout-factorial completed all 15 cases
with matching checksums and no queue disturbance. Placement did not reproduce
the previous three 8-12% regressions consistently. However branch_arithmetic
had very wide individual wall ratios (A/A 0.49-2.05), and
property_dynamic_read A/A median was 1.0251. Those facts prevent interpreting
small effects as established causes.

Before collecting follow-up data, freeze one serial diagnosis of the exact
saved T049 binaries (base 926ee634, candidate c07f0af5, plus byte-identical
base copy), 15 balanced rotating blocks. Cases: branch_arithmetic, global_read,
local_read, property_dynamic_read and hash-map. Every measured execution uses
the same literal binary path through an atomically replaced hard link; hard
link replacement happens outside the timed process. This removes concurrent
worker load and argv-path differences together; it cannot distinguish those
two causes from each other. Keep N/2N raw values and all outputs. No outliers,
no promotion verdict, no subsequent favorable-repeat search. The purpose is to
check whether the original large regression reproduces under this stronger
control; non-reproduction does not prove the old observation was all noise.

Follow-up harness correction before usable data: the first execution stopped
on POSIX rename of two hard links to the same inode (rename is a no-op).
The corrected-link run was then cancelled: replacement re-triggered macOS
launch validation (~200 ms with zero child CPU), falsely satisfying calibration
at N=1024 and making N/2N increments negative. Neither run is evidence for
engine speed. The final implementation warms and records a tiny launch after
every path replacement, before calibration or measured N/2N, and refuses a
base increment shorter than 50 ms. Keep invalid logs and partial raw data;
replace the failed protocol once, without using its timing for decisions.

## Correct hot-loop target (frozen before measuring it)

Route counters at N=262144 give exactly one loop-plan entry and zero declines
for all three original controls, with only 271 ordinary executed ops. Exact
base-binary sampling (20261016T115117Z-vm-layout-exact-control-profiles) places
836/834/835 samples respectively in the WideLoopFrame typed-loop closure;
the ordinary VM is its inclusive caller, not the repeated dispatch body.
This corrects the initial attribution; ordinary-VM displacement alone was
an insufficient hypothesis.

Freeze a final layout-only experiment on the unchanged original reader:
move only that sampled WideLoopFrame closure between its original natural
page offsets 0x5c0 and 0x7a0 (the T049 binaries differ by 480 bytes there).
Use the same audited padding method as phase 1, two builds plus an identical
copy of the first, and require every other shared text-symbol address to
match. Measure branch_arithmetic, global_read, local_read and hash-map,
15 serial balanced blocks, same literal execution path, warmed after every
replacement. Retain all raw samples and require >=50 ms base increments.
No offset search or promotion; a repeatable effect above A/A supports the
actual executor's placement as a contributor, not a universal best offset.
If the A/A control is unstable, the corresponding case remains unresolved.

## Results and conclusion

The ordinary-VM-only hypothesis was wrong for these controls. The sampled
WideLoopFrame closure's placement alone reproduces the original regression's
sign and almost all its magnitude, with unchanged reader semantics and every
other shared text-symbol address fixed. This is evidence for layout as the
principal contributor to these three regressions, not proof that every T049
change came from this single address or that a particular CPU cache caused it.

Ratios below use wall N/2N increments (hash-map is whole process), medians of
15 balanced blocks; no samples removed. A/A is a byte-identical base copy.

| Case | Exact original candidate/base | Only hot-loop 0x7a0/0x5c0 | Layout A/A |
| --- | --- | --- | --- |
| broad/branch_arithmetic | 1.085164 | 1.083914 | 1.002534 |
| broad/global_read | 1.125122 | 1.129073 | 0.999176 |
| broad/local_read | 1.080216 | 1.082748 | 1.001270 |
| external/jetstream3-js-subset/hash-map | 0.988060 | 1.001710 | 0.999427 |

The three layout effects' unadjusted paired-median bootstrap 95% intervals:
- broad/branch_arithmetic: [1.074700, 1.089857], instruction ratio 1.000033.
- broad/global_read: [1.122433, 1.131891], instruction ratio 1.000041.
- broad/local_read: [1.072570, 1.093388], instruction ratio 1.000045.

Fixing the other shared functions while varying the reader (phase 1) leaves
hash-map only 0.8-1.2% faster, with about 1.2% fewer instructions. T049 stays
rejected and reverted. Current main already has the original fast natural
placement; this diagnostic does not claim a further 8-13% speedup over main.

The current x86 order file matches only 32/625 named symbols and 0/541 runtime
symbols against exact base 926ee634. It therefore does not pin the engine as
intended. Match/address auditing and stable slots are the next maintenance
boundary; an unchecked regeneration is not an accepted optimization.

## Evidence identities and verification

Valid jobs (all exit 0, none DISTURBED):

- 20261016T113538Z-vm-layout-matrix-build-fixed: four-cell build and address audit.
- 20261016T113550Z-vm-layout-factorial: 15 cases, 15 blocks, 4 workers plus A/A.
- 20261016T113843Z-vm-layout-record-full-gate: full check at a648dc19, including
  5170 Test262 subset cases; all later changes in this task are documentation.
- 20261016T114727Z-vm-layout-control-routes: non-timing reused T049 counters;
  diagnostic executable hash checked against that job's identity.
- 20261016T115117Z-vm-layout-exact-control-profiles: untouched base sampling.
- 20261016T115221Z-vm-layout-original-serial-warm: exact original binaries,
  five cases, 15 blocks, serial, same warmed literal path.
- 20261016T115424Z-typed-layout-pair-build: two baseline-logic builds, only
  sampled closure displaced; addresses 0x10027a5c0 and 0x10027a7a0, size 13696.
- 20261016T115447Z-typed-layout-serial: four cases, 15 blocks, serial same path.

Controlled layout executable SHA-256 values (names identify requested offsets):

- `base0020`: `166739b1d48b7f836888323cfcb9f9c0dcdac651962c653043c7aa07acc60c82`.
- `base0270`: `14ec99fe3abff25a014ca3d23b0a1312bd1f6de0c5bb0d1413b41b7855318f32`.
- `cand0020`: `31b31932fd916bf693fd4fc43c3888bedffc9533b4f78214bad5f73a4280fa08`.
- `cand0270`: `ab3ad0903d13dc74654162760f6d4fecd25c62a63c6d947cbd35045c6471cd33`.
- `base05c0`: `05a93b759d9d82e81f295791090112ee902f2e56f9402d495d722e6c638d4833`.
- `base07a0`: `6dd35d1ebaa4f1e7b85a67a50c9a3a0f441958b3748d49755140051788605cfa`.

Branch CI at 1ce0a880: 38897088772 passed. The final record runs context/fast
checks and a post-integration full gate; production Rust/build inputs remain
identical to 84ceebc4. Workspace outputs/quickjs-layout-study/report.md contains
the Chinese report, paired raw-data analysis and reproducible diagnostic
scripts; outputs/imac-bench/<job>/ holds supplementary original bundles.

Protocol failures are preserved, not pooled: 113333Z build (assembly needed
subsections), 114623Z serial (same-inode rename no-op), 115013Z serial
(cancelled for launch-validation-contaminated calibration). No useful timing
was discarded from the valid runs, and none of those failures reopened T049.
