# Final S3b public86: payoff requirement missed

Job **20261101T043615Z-t077-s3b-final-public86**, candidate **8413f925**, baseline
**3cc12c6f**. Runtime is **e1405670** plus final x86 order. Five natural-layout
pairs, qualified v5 workers-4 policy, all 86 public cases; no held-out input.
Runner exit **0**, **430/430 valid attempts**, zero invalid/heavy samples.
Original verdict **likely loss**. Budget met: command **499.197 s**; end-to-end
**559.989 s**, including queue quieting. Host is iMac.lan; main's generic hostname normalization is
used with the unchanged policy and case inventory. No alias exception or guard
bypass. Queue/date checked before submission; the command guards the CST blackout.

| Lane | Cases | Wall candidate/base | Instructions candidate/base |
| --- | ---: | ---: | ---: |
| Development | 10 | 1.086527 | 1.107831 |
| External | 45 | 1.252659 | 1.275859 |
| Broad | 25 | 1.079323 | 1.095920 |
| Sentinel | 6 | 1.005211 | 1.051771 |
| Equal-case portfolio | 86 | 1.161922 | 1.184748 |

Every lane regresses in both metrics. S3b's required public payoff is **not met**;
zero retired CallEnv events do not establish a speedup. No tuning or retry follows
this result. This is diagnostic public triage, not formal acceptance; lead owns
formal, held-out, RSS and re-review, and must decide the stage's disposition.

[Summary](summary.json) computes equal-case geometric means directly from every
case's frozen wall/instruction ratio without changing [the original verdict](triage.md).
[Request](request.json), [result](triage.json), [exact command bytes](command.json), [log](log)
and [status](status) retain revisions, binaries, recipe/policy/inventory hashes,
health, all per-case ratios and stage times. Request SHA-256:
`3b58dbbef2b9a2e36b99794b15ab548c6ac8fef72bb3a0e9ac9b487de41cd6f5`;
result SHA-256:
`9f3cf994d2fa77058667ecc3063cc648203ef68c80c85f658d24bcef5479bd03`.
Candidate/base binary SHA-256 are in summary.json. No origin push or main merge.
