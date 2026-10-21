# T056 hardware attribution handoff

[Record](../../archive/T056-pmu-attribution.md) owns conclusions and limits.
**Blocked on the iMac:** no xctrace installation located; kpc getters return
EPERM and sudo requires a password. No PMU trace has been collected. Do not
submit the timing job merely to rediscover this. No security/permission change
or tool installation is authorized by this protocol.

## Evidence (non-timing)

- `capabilities.py` → `20261021T103341Z-t056-pmu-capabilities-v2` done0:
  `capabilities.json`, CPU/SDK/private-framework availability (not access).
- `catalog.py` → `20261021T103456Z-t056-pmu-catalog` done0:
  `getters.json`, local event metadata; log confirms SIP enabled. Initial
  capabilities run looked for csrutil at the wrong path; producer is now fixed and catalog uses
  `/usr/bin/csrutil`. Missing executable was not interpreted as SIP disabled.
- `rusage_smoke.c` → `20261021T104203Z-t056-pmu-rusage-smoke` done0:
  `rusage-smoke.json`, self/own-zombie process counters are readable.
- `capability_extra.py` → `20261021T105828Z-t056-pmu-extra-capabilities` done0:
  `access-extra.json`, no Xcode app in system/home Applications, sudo blocked,
  existing repository cycle-trust probe **false**. Readable rusage cycles are
  not accepted CPI evidence. No profiler was run via powermetrics.
- `fp_state.py` → `20261021T104647Z-t056-fp-state-v3` done0/6s:
  `fp-state.json` retains 24 snapshots and six full-run logical traces;
  `fp-summary.json` records semantic comparison and canonical symbol identity.
  Producer uses unsigned canonical **copies**, no resigning; asserts all file
  bytes outside two patch regions, headers and mapped sections are unchanged.
  Integer-only checkpoint bridge changes execution and then dumps/exits; it is
  never a timing/PMU image. Full control traces use existing agents/perf-counter
  diagnostic images; unmodified canonical stdout is independently compared.
  v1 failed an entry-iteration assertion; v2 failed checkpoint selection because
  an ELF-style local label left a Mach-O relocation. v3 uses Darwin local labels;
  producer now also rejects unresolved object relocations before patching.
- `stack-result.json`: complete 30-phase lead result, exact raw-file hashes,
  original automatic assessment and all phase C/B medians. No outlier removed.

## Lead-only job (prospective; at most 20 minutes)

Once an already authorized headless xctrace entry is available on the iMac,
`T056_XCTRACE_BIN` may name it and `T056_PMU_TEMPLATE` may name an existing custom
Instruments template. These variables must be present in the **remote** job
body/environment; setting a local shell variable alone does not forward it.
The default backend is `CPU Counters`, with no fallback to wall/rusage cycles.
No kpc_demo binary is bundled: self-thread kpc getters cannot measure an exec'd
child; external-process kperf sampling needs usable privileged tracing access.

Lead renders, submits with `--timing`, and immediately starts the watchdog:

```sh
python3 tasks/diagnostics/t056-pmu/render.py > /tmp/t056-pmu-attribution.sh
/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run \
  --repo "$PWD" --ref HEAD \
  --with f58c696e8379261ed726f0cea1051ed6d34f431a \
  --with 8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a \
  --with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5 \
  --name t056-pmu-attribution --timing \
  -- bash -c "$(cat /tmp/t056-pmu-attribution.sh)"
python3 tasks/diagnostics/t056-pmu/watch.py JOB_ID
```

No owner submission. `collect.py` refuses `no-timing` status, validates frozen
unit/policy, exact correctness/86-case receipts, host and three canonical hashes,
and refuses production-tree drift. No build occurs in the job. Four roles
(base, aligned 8f, dfff, separate-inode base A/A), two cases, N/2N, two paired
blocks produce 32 traces. N=23,068,672; role/case order rotates/reverses. One
active executable path, cleared old diagnostic environment; checksums must match.

`config.json` seals collector/backend/protocol/helper/window and event-plan
hashes. `event-plan.json` is an event **request/crosswalk**, not a programmed
PMU configuration. CPU Counters availability and event subset remain unverified
on this host. CLI help is checked before launching any engine. Missing tool or
unsupported CLI writes `blocked.json` with zero PMU samples. Permission failure,
empty/no counter schema, timeout or bad checksum stops without retry/zero-fill.

Each trace has a 5s recording limit; recording/export bundle ≤30s. Collection
1080s; body 1140s; runner-start guard stops at1190s, watchdog at1200s including
quieting (queue wait excluded). Own descendants only are stopped; no global
kdebug/PMU reset. Admission requires25min until the host-local quiet boundary;
03:00–05:00 and the15min margin before03:00 are excluded. Unknown xctrace
throughput means completion is not promised; partial files remain diagnostic.

Raw `.trace`, command/help output, target stdout, TOCs and counter-table XML are
retained. `capture_complete` never implies `hardware_attribution_validated`.
Before ratios, manually verify PID/executable, completed target, sampled vs
counted units, user/kernel scope, event encoding/modifiers, coverage and
multiplexing. Whole-process N/2N subtraction is only valid for comparable
additive counts; sample weights/percentages cannot be treated as counts.
CPU Counters can perturb execution; compare base A/A and canonical trend.
Absent events stay unavailable. Never consume an acceptance attempt from this.

## Local validation

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-pmu/validate.py
```

Fetch the named correctness/pre-gate outputs first; `T056_ARTIFACT_ROOT` may
select their local parent directory (default workspace outputs/imac-bench).
This validates recorded bits/ops and receipts, rendered shell/Python, simulated
32-trace collector plus blocked/empty/timeout paths, event-schema rejection and
deadlines. Simulated counter XML is **not** hardware evidence; see validation.json.
