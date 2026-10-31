# S2 primitive-read sampling receipt

Non-timing job `20261030T185822Z-t077-s2-r2-primitive-profiles`, main `dcee3c56` versus `b488bff8`.
Unmodified canonical images, separate processes: `qjs benchmarks/workloads/broad-micro.js string_slice 100000000`.
Each process was sampled with `/usr/bin/sample <own-pid> 10 1 -file <output>`; no wall-time comparison or held-out input.
Candidate has 8,350 sampled stacks; `NamedPropertyCache::read_object` accounts for 873 top-of-stack samples
(10.46%). Its own fast and full miss branches share a wrapper. This supports testing a separately inlined live-slot
hit and out-of-line cold miss handler; it does not establish a causal speedup. Base uses the removed exact-value cache.

| artifact | SHA-256 |
| --- | --- |
| base sample | 502844f5f067606705a39ec78dd7b5688a3794746ac8479b2df012245bf80188 |
| candidate sample | d5aa853ced9a1c027d709edb1eba54326d9be302e4fe0fba69e1abcbd6c882e8 |
| base000/qjs | 6fc7251582f597f2e85cc8ce9e3a37cb7aaf9a32c442ab03a2abdaccb1204d3b |
| cand000/qjs | 3546331540afa074ed239095e5d1ec8b553ee4587bcbb807095d784929e4c8e8 |
| workloads/broad-micro.js | e4c79e512f11bd3716cba0f2a85d650cc120594b6f06370acd92f74952c3248c |

Sampling conclusions and image/workload hashes are recorded here; raw logs remain in the named iMac job.
Instruction-counter follow-up must use a new candidate revision. No cached value, descriptor bypass, or unsafe is admitted.
