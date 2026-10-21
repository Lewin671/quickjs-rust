# T056 PMU path closed

- Status: closed by lead decision; no further helper calls, retries or repairs.
- Implementation: final v3 SHA4b5776d1 remains installed; owner changed no
  system state. [Helper README](../diagnostics/t056-pmu/kpc-tool/README.md)
  contains the optional user-only uninstall commands.
- Verified at: installed v3 root path/UID drop/readback/cleanup passed lead smoke;
  retry collector0a7bb6e5 failed lead174256Z after6s. Engine still dfff3bb6.
- Evidence: [closure receipt](../diagnostics/t056-pmu/kpc-tool/closure.json),
  [all final raw rows](../diagnostics/t056-pmu/kpc-tool/run-v3-retry-final.jsonl),
  [initial run](T056-kpc-collection-retry.md), [v3 preparation](T056-kpc-counting.md).
- Unresolved: exact width/shadow/PMI/client cause; final binary exposes no
  end/window event configuration. No reliable mechanism-counter result.
- Next action: no PMU continuation. Follow the pragmatic stage-A candidate;
  retain kernel isolation as the M2 follow-up. No reinstall or sudo action.

v3 did establish working root KPC counting without owning ktrace, correct
nonroot execution, fixed environment, exact setup readback, per-CPU snapshots,
matched idle controls and clean release. The lead's short Perl smoke reported
about0.1% idle. That is not transferable to another window: first-run base
N/2N had6.46%/7.51% normalized idle cycles and a negative cycle increment.
None of those unfavorable valid metrics was retried or promoted as evidence.

`20261021T174256Z-t056-kpc-counters` at0a7bb6e5, tailspin enabled, reached the
first base/global_read/N unit only:

| Pair attempt | Helper exit | Observed rejection | Cleanup |
| --- | ---: | --- | --- |
| 1 | 1 | read.delta_integrity; per-CPU discontinuity | true |
| 2 | 0 | collector caught huge positive jumps despite helper monotonicity | true |
| 3 | 1 | read.delta_integrity plus idle/work1.079727; structure assertion | true |

The first2 were recorded as invalid counter observations and the same unit
restarted. The third was preserved in raw.jsonl, but not observations.jsonl:
structure() rejected its idle/work ratio outside0.95–1.05 before recovery
classification. Do not call this a complete3-attempt recovery receipt, count
it as a timing result, or relabel any sample valid. The collector's schema/
window gate remained strict, but its invalid-observation audit is incomplete
for this combined failure. No further patch or helper invocation follows.

Per-CPU values again jump toward2^48+x. The lead's leading hypothesis is
another kpc/kperf client reprogramming counters with PMI reload values,
possibly re-enabled tailspin. It is **not confirmed client attribution**:
only setup configuration is reported, so neither that client nor the exact
reload/normalization path can be proved. Ordinary modulo-wrap correction was
not justified by the first run and is not applied now. The pragmatic decision
is that this host/protocol cannot deliver reliable PMU data within the user's
no-more-sudo/reinstall constraint, not that a particular daemon is proven faulty.

The helper and digest-bound rule remain installed; tailspin remains enabled.
Owner executes neither the helper nor uninstall/system commands. Historical
scripts/receipts stay for diagnosis and are not instructions to resume PMU.
No acceptance attempt was consumed; the last attempt remains for the unchanged
frozen stage A+B contract.
