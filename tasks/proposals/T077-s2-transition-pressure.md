# S2 design correction: persistent failed-sharing pressure

Frozen before implementation, following the first roots/maps instruction job.
Candidate `90719950`, base `dcee3c56`, job
`20261030T190528Z-t077-s2-r2-construction-roots-maps` reports graphlib
9,639,476,090 versus 8,989,084,835 median plain instructions (1.0724),
223,734 shape misses/nodes, 25,559 hits and 3,552,641 allocator requests.
The live-fanout policy only removes a small part of the 239,939 before misses.

The live count forgets pressure when ephemeral child layouts disappear. Replace
that admission test with one saturating append-miss byte per layout. A computed
addition with no live cached child enters a private dictionary after more than
eight append misses at that layout (the existing small transition capacity).
A known live transition remains shared. Stable named/class/literal/JSON plans
retain their shared construction policy. Only transition append misses advance
the byte; it retains no key IDs, leases or script values. Layout destruction
naturally resets its history. No global mutable state or value cache.

This changes storage admission, never descriptors/order/prototypes. It avoids
repeatedly allocating unshared paths after the shared keep-alive budget is
consumed; weak transition correctness and the byte bound stay unchanged.
Verify known-edge sharing, ephemeral-edge churn, dictionary order/descriptors,
all semantic gates and a fresh same-input count cohort before claiming payoff.
No timing during the iMac quiet window; array size work is still queued.
