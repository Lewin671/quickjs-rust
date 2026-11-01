# Final S3b public86 after the global-store repair

Job **20261101T140707Z-t077-s3b-final3**, candidate **8abd4c69** (runtime
**0148b2eb** plus the regenerated x86 order), baseline **3cc12c6f**. Five
natural-layout pairs, qualified v5 workers-4 policy, all 86 public cases; no
held-out input. The iMac queue was idle and outside the CST blackout at
submission. Runner exit **0**, **430/430 valid attempts**, zero invalid/heavy
samples, budget met (command 495 s). Original verdict **likely loss**.

| Lane | Cases | Wall candidate/base | Instructions candidate/base |
| --- | ---: | ---: | ---: |
| Broad | 25 | 1.0165 | 1.0327 |
| Development | 10 | 1.0342 | 1.0190 |
| External | 45 | 1.0255 | 1.0299 |
| Sentinel | 6 | 0.9910 | 1.0321 |
| Equal-case portfolio | 86 | 1.0214 | 1.0296 |

[Summary](summary.json) takes unweighted geometric means of the frozen per-case
ratios in [triage.md](triage.md); it does not change the original verdict.
Against the 689efca2 job `20261101T123235Z-t077-s3b-final2` (same base binary),
per-case instruction ratios move by 1.0030 overall (broad 1.0015, development
1.0074, external 1.0033, sentinel 1.0000): the global-store repair plus the
regenerated order. The largest remaining cases match the recorded S3c
obligations and residual classes
([progress](../T077-s3b-progress.md#round-4-milestone-3-global-store-repair-and-final-requalification)):
lodash 1.193/1.086, captured_write 1.158/1.188, string-unpack-code 1.169/1.042,
binary-trees 1.119/1.153, raytrace-public-class-fields 1.152/1.153.

[Request](request.json), [result](triage.json), [command](command.sh), [log](log)
and [status](status) retain revisions, binaries, policy hashes, health, every
case ratio and stage times. Diagnostic public triage, not formal acceptance; the
lead owns formal, held-out, RSS and the stage decision. No origin push or main
merge.
