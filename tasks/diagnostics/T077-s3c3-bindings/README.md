# S3c C3 captured-write cut (C3 remains incomplete)

The runtime source of this commit is pinned by [builds](builds.json); binaries
and source are also bound in the original [call slopes](call-cost.json),
[controls](controls.json), [request](control-request.json) and
[method coverage](method-coverage.json). Comparison is S3b 8322fc98, parent is
the depth repair 098effa5. These local instruction/coverage runs are not timing.

The compiler marks non-global, non-dynamic var/simple-parameter declarations.
ParentLocal/ParentUpvalue links propagate proofs; Named links never do. Closure
linking confirms metadata provenance, name, mutable kind and initialized state.
Only a nonzero bounded 128-capture mask allocates one immutable box per closure;
FunctionData remains <=240 bytes, avoiding the larger Rc allocation class of
the rejected inline-u128 representation. Received indices are compiled into
wide loads/stores; no per-op slot-to-capture lookup remains. A fast write uses
one safe RefCell replacement, with scalar ownership handling and publication
before resource release. TDZ/const/import/NFE/object/default-parameter/dynamic
routes keep checked assignment. No cell identity or value cache is introduced.

| Call case | S3b instructions/call | C3 cut | Saved |
| --- | ---: | ---: | ---: |
| upw0 | 1,002.55 | 910.56 | 91.99 |
| upw1 | 1,333.57 | 1,157.53 | 176.04 |
| upw4 | 2,275.18 | 1,865.32 | 409.86 |

The required >=70/call is met in all three. Counters preserve 1M calls,
9M/13M/25M compact ops, 30 general ops and 105 cells; fast writes are
0/1M/4M. Compared with C2's 931.96/1267.12/2242.18, subtracting the zero-write
improvement gives 88.2/88.9 saved per additional captured += operation.
That is a paired read/write protocol measurement, including the newly indexed
read; it does not isolate the machine cost of BindingCell replacement alone.

The receiver is borrowed directly from its immutable indexed prefix within a
segment; full/lexical roots retain semantic routing. Prefix references end before
stack growth or release. The initialized-receiver accessor consumes CallRoutes;
lexical roots establish their routes before the first read. Native leaf arguments
and Context temporaries are outlined from the typed pure-call path.

Coverage against S3b proves identical IC reach on hash-map: 8,100,131 hot named
reads, 1,080,049 hot writes; 180,000 closed-form and 2,160,029 compact calls.
The old C2 receiver facade was read 5,040,148 times. Receiver routing recovers
most of the C2 loss; remaining hash-map instructions **1.003722** and broad
method_call **1.002384** still miss <=1.00. Fannkuch 0.999771, tofte 0.978666 and
the supplied method repro 0.975248 pass. No local gate is waived. Scope/reference
boundary Ops, indexed eval reads/general cell stores, complete C3 measurements,
the one C3 iMac job, C4 and C5 are still outstanding.

Default runtime tests pass **2,735**, including two new provenance/alias/error
tests. The commit's fast gate records current feature/lint/selected Test262 checks.
The prior depth repair's [full gate](../T077-s3c3-depth/checks.json) passed all
5,174 curated cases; C5 still requires a new full gate on its frozen source.
