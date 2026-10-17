# T038: Same-image Gaussian blur engine finding

- Status: historical-campaign; diagnostic measurements, no engine fix.
- Implementation: none; no benchmark-specific runtime or measurement exception.
- Verified at: source4c9f959e, collector3c082e6a for the new trial below.
- Evidence: exact jobs/images and measurements below; full raw artifacts retained.
- Unresolved: physical cause and any shared mechanism with DFT variance.
- Next action: [T038](../T038-audio-dft-variance.md) owns the engine investigation.

## Related engine finding: same-image Gaussian blur wall tail

T055 feedback-2 explicitly classifies this as an engine finding, not a
measurement-rule exception. Source 4c9f959e, image
c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e,
canonical job 20261017T022617Z-decision-rule-b-formal30-whole (workers-4).
Candidate and base are byte-identical; the host log has zero heavy samples.
Kraken imaging-gaussian-blur successful process wall times:

| Role | ok / attempted | median s | max ok s | max / median - 1 |
| --- | --- | ---: | ---: | ---: |
| candidate | 29 / 30 | 11.262680115 | 13.858422675 | 23.05% |
| base | 30 / 30 | 11.213226504 | 13.341806271 | 18.98% |
| pinned NG | 30 / 30 | 10.274794007 | 10.765107934 | 4.77% |

Candidate block 27 additionally hits the 15-second timeout at 15.021374156 s;
that observation is right-censored, at least 33.37% above its successful
median. Do not treat it as an ok duration, delete it, or use the incomplete
formal bundle for a promotion. The raw record and exact failure are preserved
in [T055's prior qualification record](T055-feedback-1-qualification.md).
The engine's roughly 20-35% upper tail versus NG's roughly 5% is plausibly
related to the DFT execution-variance family; no shared cause is established.
No sampled stack/scratch addresses, intermediate floating-point exception
state, continuous thermal telemetry or scheduler attribution accompany this
Gaussian observation. Investigate it later as an engine issue with appropriate
controlled evidence. T055's reference-derived hang timeout is uniform across
cases/roles and does not remove the tail or waive the 3% precision gate.


## Full uncensored trial under the uniform timeout protocol

Job 20261017T050401Z-decision-rule-b-feedback2-formal30, source4c9f959e,
collector3c082e6a; same pinned images as above, fixed equal-length role/hash
paths. All 30 samples per role complete under the frozen 60-second limit;
zero heavy monitor samples. No sample is deleted or replaced.

| Role | ok / attempted | median s | max s | max / median - 1 |
| --- | --- | ---: | ---: | ---: |
| candidate | 30 / 30 | 11.296645371 | 11.745100300 | 3.97% |
| base | 30 / 30 | 11.311408630 | 15.693555132 | 38.74% |
| pinned NG | 30 / 30 | 10.219048457 | 10.679947143 | 4.51% |

The long tail persists on a byte-identical engine image and switches which
role shows the larger observed tail. The generic hang limit retains the
15.694-second observation; it does not remove variance. Different run/time and
path protocol prevent causal attribution to any one changed condition. Treat
the possible DFT relationship as a hypothesis for later engine investigation.
This Gaussian diagnostic is separate from the new formal bundle's failed
bitops case precision and never waives that qualification failure.
