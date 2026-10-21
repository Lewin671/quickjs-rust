# PMU path closed — historical tooling only

The lead closed this path after bounded-retry run174256Z failed within6s.
Do not call the helper, run these collectors/renderers, retry the experiment,
reinstall or request another sudo/daemon action. See the
[closure record](../../../archive/T056-pmu-closure.md) for what worked, failed
and remains unproved. [Final raw rows](run-v3-retry-final.jsonl) and
[closure receipt](closure.json) retain the outcome honestly.

Root v3 counting, privilege drop, setup readback, idle control and cleanup
worked in short smoke. Repeated per-CPU2^48-scale discontinuities prevented
reliable capture; the third attempt also failed the idle/work ratio gate
before invalid-observation audit. The raw result remains. No end configuration
is available, so another client/PMI reload is a hypothesis, not proven tailspin
attribution. No count correction, timing conclusion or acceptance attempt.

The final helper remains installed at `/usr/local/libexec/qjs-kpc`, SHA-256
`4b5776d1126490ddd9d3b6b8394cbe9b43c3406752534fb0704c501cfb81eafe`,
with its rule at `/private/etc/sudoers.d/qjs-kpc`. Tailspin is enabled. The
owner performs no further privileged action. If the user later chooses to
uninstall it, the manual commands are:

```sh
sudo /bin/rm -f /private/etc/sudoers.d/qjs-kpc /usr/local/libexec/qjs-kpc
sudo /usr/sbin/visudo -cf /private/etc/sudoers
```

Historical preparation: [v1](../../../archive/T056-kpc-tool.md),
[v2](../../../archive/T056-kpc-setup-failure.md),
[v3](../../../archive/T056-kpc-counting.md),
[collector recovery](../../../archive/T056-kpc-collection-retry.md).
Their frozen configurations, source qualifications, validation and raw evidence
are preserved; they do not authorize renewed measurement. Continue with the
[pragmatic stage-A task](../../../T056-interpreter-core-v2.md).
