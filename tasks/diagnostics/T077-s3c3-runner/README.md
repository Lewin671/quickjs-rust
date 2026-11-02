# C3 single runner checkpoint

Job **20261102T054839Z-t077-s3c2-c3**, source **bc17ccea**, base **8322fc98**.
The queue was idle and submission was outside UTC 18:45–21:00. The only ref
transfer was to the runner. Original request, command, log, status and triage
files are retained unchanged; [summary](summary.json) derives lane geomeans.

**430/430 valid**, no invalid or heavy samples, budget met (498.7 s command,
559.5 s including quieting). The raw verdict **likely loss** is retained.
The lead's C3 external instruction gate passes; this is exploratory public
triage, not formal, held-out or RSS acceptance.

| Lane | Wall C3/S3b | Instructions C3/S3b |
| --- | ---: | ---: |
| development | 0.958059 | 0.981809987 |
| external | 0.995802 | **0.995818290** |
| broad | 0.940823 | 0.946315193 |
| sentinel | 0.957312 | 0.953208713 |
| selection | 0.972427 | 0.976567690 |

Hash-map instructions **0.991951550**, method_call **0.596515895** both meet
the <=1 requirement. Fannkuch is **1.001624621** on x86 (wall 0.977924);
the excess is retained and requires C5 recheck, not rounding or a waiver.
[Local follow-up](fannkuch-followup.json) preserves all three five-pair sets
using identical binaries: their post-hoc median ratio is **1.000075329**.
That diagnostic is not a replacement strict gate or an acceptance claim.

The [local C3 receipt](../T077-s3c3-final/README.md) owns the complete 109-case
measurements, correctness and depth. C4 now proceeds under decision 3. C5 must
finish the complete matrix, regenerate x86 order and freeze runtime source
before the single final public86. The lead owns formal/held-out/RSS and review.
