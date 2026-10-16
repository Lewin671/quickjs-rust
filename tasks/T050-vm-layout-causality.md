# T050: Ordinary VM layout causality diagnosis

- Status: active.
- Implementation: none; diagnostic binaries only, no production change planned.
- Verified at: base 84ceebc4944fa2ecf5f1f4c46ebe98e8282bafdf; experiment not run yet.
- Evidence: this task records the frozen diagnostic design; results pending.
- Unresolved: whether T049 control regressions follow VM placement or property-read logic.
- Next action: build and audit the diagnostic matrix on iMac, then measure once.

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
