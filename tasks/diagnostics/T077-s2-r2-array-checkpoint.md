# S2 array checkpoint at cdda4bd4

This completed checkpoint predates live-slot refinements, the typed Date slot and restored numeric stores. It is not final acceptance.

Verification: full local gate 2,465 runtime/5,170 curated; cached-versus-generic agents/perf-counters 2,512 pass on arm64 and x86_64.
NG fixtures: 266 each in default/off/verified modes. Instantiated chain differential: 300 original and 50 warmed-branch seeds, all three policies identical (1,050 successful processes).

Cached/off complete exact scans execute 42,672 configured cases each, zero failures/timeouts/gaps or lost passes.
All 53,572 case records match prior complete scans, SHA-256 `2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.
Pinned release CLI SHA-256 `4e0b5184cee3cbce4d9c2f4d4a0963759523b2dd508a1d3098cb1affa5a51f2b`.
Cached ledger row is cdda4bd4; off row uses documentation-only descendant 1992d40b with identical runtime/CLI/harness inputs. Both complete scans have immutable appended rows.

Public natural-layout timing: job `20261030T210813Z-t077-s2-r2-public86-triage`, all 86 policy selection cases, no held-out/custom target. Five paired samples. Verdict **likely loss**, diagnostic only.
[Raw triage](T077-s2-r2-public86-cdda.json) retains exploratory intervals, build/host/workload/packet hashes and health.

| lane | cases | wall geomean | instruction geomean |
| --- | ---: | ---: | ---: |
| development | 10 | 0.970817 | 0.962470 |
| external | 45 | 1.036402 | 1.026964 |
| broad | 25 | 0.997677 | 0.992213 |
| sentinel | 6 | 0.943657 | 0.939332 |
| all | 86 | 1.010599 | 1.002839 |

Resource job `20261030T205608Z-t077-s2-r2-array-x86-rss`, three base/candidate pairs per original 300k-retained-receiver script; counts/RSS only, not formal timing or >=15-pair resource acceptance.
[Raw RSS](T077-s2-r2-array-rss.json) records every sample and input/image hash. Medians are bytes.

| case | main median | candidate median | ratio |
| --- | ---: | ---: | ---: |
| arr_mem | 42008576 | 42393600 | 1.009165 |
| arr_mem_nocall | 41971712 | 42369024 | 1.009466 |
| fn_mem | 120180736 | 130351104 | 1.084626 |
| fn_mem_nocall | 110481408 | 130342912 | 1.179772 |

The default-array RSS regression is reduced to about 0.9% over main, from the prior 18-24%. Function controls remain a material cost, especially no-call retained functions (+18.0%). No waiver or formal resource claim.

Live size assertions: ArrayData/ArrayColdData 56/104, NamedStore/RefCell 64/72, ObjectData 88 at this checkpoint (later Date change makes it 96 nominal), FunctionData 320, Value 16, IC entry/handle 32/8, production string payload 72.
No dependency, unsafe, parser/AST or binding-snapshot change. Later source still requires full exit checks and payoff.
