# S3b public86 on 689efca2 (superseded)

Job **20261101T123235Z-t077-s3b-final2**, candidate **689efca2** (runtime
**f7057a51** plus the regenerated x86 order), baseline **3cc12c6f**. Five
natural-layout pairs, qualified v5 workers-4 policy, all 86 public cases; no
held-out input. The iMac queue was idle and outside the CST blackout at
submission. Runner exit **0**, **430/430 valid attempts**, zero invalid/heavy
samples, budget met (command 494.7 s, end-to-end 555.4 s). Original verdict
**likely loss**. The re-review then found global-store regressions in this
runtime; [job `20261101T140707Z-t077-s3b-final3`](../T077-s3b-final3-public86/README.md)
on the repaired 8abd4c69 supersedes this record, kept as its comparison point.

| Lane | Cases | Wall candidate/base | Instructions candidate/base |
| --- | ---: | ---: | ---: |
| Broad | 25 | 1.0158 | 1.0312 |
| Development | 10 | 1.0289 | 1.0115 |
| External | 45 | 1.0296 | 1.0265 |
| Sentinel | 6 | 0.9799 | 1.0321 |
| Equal-case portfolio | 86 | 1.0220 | 1.0265 |

[Summary](summary.json) takes unweighted geometric means of the frozen per-case
ratios in [triage.md](triage.md); it does not change the original verdict. The
largest remaining cases match the recorded S3c obligations and residual classes
([progress](../T077-s3b-progress.md#round-4-milestone-3-global-store-repair-and-final-requalification)):
lodash 1.174/1.070, captured_write 1.143/1.164, string-unpack-code 1.163/1.041,
raytrace-public-class-fields 1.151/1.139, binary-trees 1.116/1.139,
top_level_function_call 1.075/1.124. branch_arithmetic falls from 1.48 to
1.014 wall (instructions 1.015) with the regenerated order.

[Request](request.json), [result](triage.json), [command](command.sh), [log](log)
and [status](status) retain revisions, binaries, policy hashes, health, every
case ratio and stage times. Diagnostic public triage, not formal acceptance; the
lead owns formal, held-out, RSS and the stage decision. No origin push or main
merge.
