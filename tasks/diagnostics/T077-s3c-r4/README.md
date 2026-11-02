# S3c round 4: recursive native-frame repair

Base of repair: c80ac690; accepted depth reference: 8322fc98. Runtime
semantics and tier admission are unchanged. The dormant multi-frame branch
is outlined from run_completion, so its full inline-frame move temporaries
do not remain on every production recursion level. No extra allocation or
stack-policy change. The direct-leaf entry/fallback split remains unchanged.
A debug-only assertion checks active binding_scope against pc_scopes before
ordinary operations, including control transfers; checkpoints install scopes
before the next ordinary operation. A negative unit test proves detection.

The permanent depth collector now covers all 14 paths (the original eight
plus try/catch, construct, spread, arguments, super and general VM), defaults
to the full inventory, and --baseline rejects any lost depth or inventory drift.
Qualification is in progress; current measurements are not final acceptance.

Exact cached/IC-off Test262 is not repeated for this frame-layout-only repair:
no JS operation, frame contents, scope transition, admission or binding policy
changes; the existing complete production scans each pass 42,672 configured
cases. The debug assertion adds observation only. Round4 still repeats the
requested stress, semantic and depth gates; source-pinned results follow.

Pre-existing reproducers are retained in [s3c-findings](s3c-findings/). They
remain out of scope: nested finally/continue skips outer finally, duplicate
sloppy parameters select the first argument, native stack overflow aborts
instead of RangeError (S3d). Lead round4 also reports a healthy public86 on
c80ac690: development wall/instructions 0.962/0.985, external 0.988/0.997,
broad wall 0.950, sentinel wall 0.956. These are lead-reported predecessor
results, not a performance claim for this repair.
