# T038: Audio DFT one-factor diagnostic data

- Status: historical-campaign. Completed diagnostic series.
- Implementation: none; no engine or benchmark protocol change.
- Verified at: integration `8e208b8a`, 2026-10-12, serialized process samples.
- Evidence: numbers below; raw target/audio-dft-diagnostic/ artifacts are local,
  not retained. Conclusions belong in [T038](../T038-audio-dft-variance.md).
- Unresolved: see T038.
- Next action: see T038; this record is frozen.

## One-factor experiments

Each cell is cycle median [minimum, maximum] G, 20 samples per executable per
variant; environment, cwd, binary path and argv0 are separate experiments.
Changing cwd keeps absolute binary/workload paths and inherited env fixed.
The long binary path is a byte-identical copy; argv0 alone uses Popen's
explicit executable. Environment pads are 0 / 4096 / 16384 bytes plus one
fixed variable name, with variant order rotated within each block.

| Knob | Base | Integration |
| --- | --- | --- |
| Env +0 | 2.665 [2.132,3.171] | 2.714 [2.600,3.014] |
| Env +4096 | 2.599 [1.897,3.272] | 2.733 [2.536,3.351] |
| Env +16384 | 2.610 [1.873,2.951] | 2.655 [2.552,3.298] |
| Short cwd | 2.695 [2.561,3.477] | 2.625 [2.567,3.011] |
| Long cwd (+180 chars) | 2.965 [2.593,3.853] | 2.708 [2.611,3.552] |
| Short executable path | 2.768 [2.011,3.719] | 2.779 [2.574,3.817] |
| Long executable path (+180 chars) | 2.886 [2.554,3.799] | 2.779 [2.547,3.865] |
| Short argv0 | 2.694 [2.578,4.630] | 2.689 [2.532,3.615] |
| Long argv0 (+180 chars) | 2.662 [2.546,3.584] | 2.666 [1.987,3.114] |

No knob eliminates the spread. Base +4096/+16384 produces 4/20 and 6/20
samples below 2.4G, respectively, while baseline has none. Layout sensitivity
is plausible; these experiments do not establish that any knob determines
cost. Choosing a favorable environment would conceal the issue.


## ASLR method

ASLR was tested with XNU's private per-process POSIX_SPAWN_DISABLE_ASLR flag
(SETEXEC launcher; no system/SIP change). Code and stack slide suppression
was empirically verified; malloc region bases remain randomized. This is not
an advertised public macOS switch. Stock on/off uses the same launcher, so
its fixed startup work is included in both; do not compare those instructions
directly to direct-spawn samples. On/off base medians/ranges are
2.735 [2.565,3.032] / 2.658 [2.607,2.956]G; integration is
2.805 [2.548,3.582] / 2.697 [2.545,3.111]G (20 each). Disabling ASLR
alone is not a remedy. Flag definition: [XNU](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/spawn.h).


## Startup stack checks

The entry-stack follow-up prints a main-frame marker only after evaluation,
so logging cannot change allocations before DFT. At fixed ASLR-off, env pad
0/16/32/48 bytes (20 each) shifts the main marker in 16-byte steps; the hot
bank is always exactly main + 0xd70 modulo 4096. Main itself takes adjacent
16-byte positions at fixed user env, confirming a startup stack origin,
rather than a DFT tier switch. A second env series (64/80/96, 20 each) retains
this relation; its inherited env is 24 bytes larger, explicitly recorded in
raw env_bytes, so cross-series offsets are not treated as one-factor pairs.


XNU's exec_add_entropy_key uses variable-width hexadecimal entropy strings
for the apple[] startup vector, even independently of the ASLR flag:
[XNU kern_exec.c](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_exec.c).
This could move a 16-byte-rounded entry stack. It is a source-based hypothesis,
not an exact-kernel causal attribution. A standalone C-main probe (80 ASLR-off
processes) sees a constant 406 bytes of visible apple[] strings and one stack
position, 0x4c8 mod 4096. Guard/malloc payloads are not visible at main and
ptr_munge has no value; only lengths were recorded, never entropy values.
That probe does not reveal the kernel's original string lengths and does not
confirm the proposed trigger. The Rust main/DFT address relation is direct
engine evidence; the precise OS trigger and hardware stall remain open.

## Diagnostic paired intervals

The same paired bootstrap as the external reporter gives diagnostic cycle
half-widths: baseline 40 = 5.104%, no-polling 40 = 7.701%, reverse-order 40 =
2.588%, 100-transform 20 = 1.359%. All samples are retained. The last has too
few blocks and changes the frozen workload; none substitutes for promotion.
Longer in-process repetition merits a separate protocol experiment, but a
single favorable order/interval is not evidence that the infrastructure is
fixed. No selective extension, outlier removal or pooling was performed.

