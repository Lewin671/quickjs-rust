# T056 installed PMU helper setup failure and v2

- Status: v1 root setup failed; v2 prepared, awaiting user reinstall/root smoke.
- Implementation: [v2 helper/protocol](../diagnostics/t056-pmu/kpc-tool/README.md).
  Engine/recipe/frozen unit/last attempt unchanged.
- Verified at: parent5b58ab53/engine dfff3bb6; v1 lead153553Z failed first check;
  authorized old-binary root probe153934Z done0 reproduced both paths;
  v2 nonroot155348Z done0/11s, groups155433Z done0, installer155544Z visudoOK.
- Evidence: [v1 failure](../diagnostics/t056-pmu/kpc-tool/failure-v1.json),
  [v2 build](../diagnostics/t056-pmu/kpc-tool/qualification-v2.json),
  [v2 groups](../diagnostics/t056-pmu/kpc-tool/groups-v2.json),
  [v2 validation](../diagnostics/t056-pmu/kpc-tool/validation-v2.json).
- Unresolved: old JSON cannot identify the exact failing pre-verification API;
  fixed-control mismatch is a hypothesis. v2 root behavior not verified.
- Next action: user replaces helper+digest with reviewed v2; owner may run the
  prepared short non-timing smoke. Lead timing only after successful default
  smoke. No automatic install, security change, timing or attempt consumption.

## What the installed binary proves

The user installed v1 root:wheel0755 and its digest-bound rule0440; visudo passed.
Lead job `20261021T153553Z-t056-kpc-counters` failed first `/usr/bin/id -u`
check after2s. Wrapper exit1; helper210.368ms, child137, config_verified=false,
empty stdout/threads/records. sudo and the root parent run; the child gate did
not open. `trace_drain` cannot explain config_verified=false: that flag is set
before release and before any drain. No engine or valid PMU sample ran.

Within the newly authorized boundary, non-timing job
`20261021T153934Z-t056-kpc-installed-probe` used ONLY installed SHAe3d2bc4c… .
Nonroot/root descriptors agree. Root fixed-only class1/config0x22 and mixed
class3/config[0x22,0x410879,0,0,0] both fail at the same observable boundary,
210.200/210.339ms, cleanup_ok=true and no target output. These elapsed values
are setup diagnostics, not workload timing. Public post-state sysctl showed
ktrace.state1/owner64489 (the exited helper); no global reset was attempted.
The kernel can lazily release a dead owner; this is not proof of a live lock.

Old JSON omits API/errno/actual readback, so it cannot distinguish idle-policy,
KDEBUG setup, pre-force config read, force/set failure or strict comparison.
Do not assert that sudo, trace_drain, a particular errno or fixed readback was
confirmed as root cause. Further exact diagnosis needs new diagnostic code;
old digest policy correctly prevents running a replacement as root.

## Confirmed source defects and fixes

Monterey [kperfbsd.c](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.140.41/osfmk/kperf/kperfbsd.c)
configures KTRACE_KPERF for setter requests; its reset request resets that mask.
[kdebug.c](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.140.41/bsd/kern/kdebug.c)
KDREMOVE resets KTRACE_KDEBUG only. Old cleanup unconditionally called
kperf_sample_set(0)/lightweight_pet_set(0), even when setup had failed before
any KPERF write, then removed only KDEBUG. Cleanup could itself acquire/retain
KPERF ownership. On a successful setup it also left action/timer slots nonzero,
which would fail the next idle check. These are confirmed lifecycle defects;
they are not retroactively labelled the first setup failure's exact cause.

v2 tracks its first KPERF write attempt, never creates KPERF state solely to
clean a KDEBUG-only failure, resets its own KPERF state, removes its own KDEBUG
session and verifies its ownership no longer remains. Foreign owners remain
untouched. Inert uninitialized buffers are not called active merely because
nolog=0; initialized active buffers are still rejected. API fault-injection
tests execute these ownership/state transitions, including two consecutive
sessions and no writes/resets to a foreign owner.

Every setup/drain step now names the failing API/policy, return code and errno
captured immediately after that call. Actual readback and buffer state are
retained. Cleanup has separate first-error fields and cannot overwrite setup
failure. Tests cover syscall failure, positive policy EPROTO, errno-bearing
zero-mask getter errors, readback mismatch and separate cleanup failure.

v1 manually treated KPC's fixed config word as IA32_FIXED_CTR_CTRL0x22. Hardware
counter counts/register counts and KPEP descriptors did not validate that private
ABI as root. v2 default delegates cycles/instructions to KPEP's existing named
architectural *_P fallbacks (3C/00 and C0/00), all programmable USER-only. Exact
EventCode/UMask/CMask/edge and readback remain required. No silent relaxation of
readback or OS-counting scope. Explicit `--fixed-diagnostic` preserves the old
path for a controlled post-install comparison, without another rebuild. This
is a portable candidate repair for the suspect assumption, not proof of what
the old kernel returned. JSON cycles/instructions follow actual event indices;
serialization regression tests detect swapping programmable indices0/1.

Coverage now divides by the actual child window (gate release to observation
of completion), not helper setup+cleanup. The old210ms setup interval contained
no child execution. A helper-overhead denominator would reject valid short
workloads even if PET covered nearly the entire command. No completeness claim:
all_threads_covered/full_lifetime_counts stayfalse.

## Nonroot verification and revised protocol

Job155348Z built SHA-256
`a8021d9adc7a0c583f32920832fafd34450fe462476b4d2f726a206a83f06673`
with clang14/CLT warnings-as-errors/PIE/stack protector. Seven module static
analyses passed; existing UID/FD/parser tests, new trace fault-injection and
JSON-index/error tests passed. Both default2+2 and explicit fixed2+4 descriptions
pass. No rebuilt binary was run as root; no sudoers or installed file changed.
Old qualification/groups/config/validation evidence remains frozen, v2 gets
separate snapshots. Failed intermediate154750Z had a macro-shadowing warning;
it was corrected before the final passing build.

21 supported events now use11 default groups: cycles/instructions take2 of4
programmable slots. Six sources+two AA clones, global/local, one paired block,
N23,068,672/2N yield176 units/352 measurements+2 uid/env checks. This prospective
short diagnostic has no formal precision/acceptance standing. Existing engine
correctness/86-case receipts and source/hash constraints remain. Counter mode,
indices/registers/child-window coverage are checked. Negative rare increments
stay uninterpretable. No rejected row, offset or acceptance attempt is selected.

Budget includes prior workload throughput plus0.211s helper setup for each of352
calls:254.431s;2x+120=628.862s<720 collection. Body840/runner890/watch900 include
quieting; local03–05/15min-before03 exclusion and20min admission unchanged.
Actual successful root-throughput remains unverified; timeout retains partial data.

Installer job155544Z passed nonroot visudo parsing and shell syntax. It contains
only the new fixed-path binary/rule update; user password still required for
installation. The old SHA's NOPASSWD grant cannot authorize the new executable.
After user reinstall, short `render.py smoke` runs id through default, default+
DSB and explicit fixed paths, records stage/readback results, stops on cleanup
failure and never runs a benchmark. Owner may submit only that no-timing probe.
The ≤15min measurement remains lead-only and waits for successful default smoke.

Repository `check.sh` passed including5170 curated Test262 cases on this
diagnostic working tree (engine stilldfff3bb6). Focused v2 validation and staged
fast context gate pass before handoff. No engine source,
policy or final acceptance attempt is changed.
