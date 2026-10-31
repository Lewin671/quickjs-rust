# S2 round4 structural correction

Lead authority: feedback-s2-4, replacing the previous no-cached-Value rule.
Base runtime/order 2cccf17d; baseline dcee3c56. No held-out collection.

1. PrototypeState tracks at most four successful value-write invalidations.
   NamedStore's write barrier checks prototype mode; stable mode invalidates
   the existing iterative descendant graph before every slot/accessor write.
   The fourth invalidation permanently disables value constants for that holder.
   Later value writes preserve cells; structural edits always invalidate.
   All slot mutation entry points converge on the barrier, including borrowed
   mutable string updates. Construction recipes only admit non-prototype layouts.
2. Inherited data/accessor entries may retain constants only for stable holders.
   Numbers/booleans/null/undefined are copied; every RC payload is weak, including
   string/BigInt/accessor halves. Receiver layout + chain generation guards the
   constant; failed weak upgrade is a miss. Own data stays live. The holder's
   weak identity remains for verification and live fallback, never ownership.
3. Split hot guard/hit APIs from cold non-inlined miss/update/verification and
   megamorphic handling in all consumers. Report exact-image address-based static
   instruction counts for the previous profiled typed VM/Wide and wide dispatcher.
4. Attribute raytrace/gaussian extra instructions with counter builds of both
   endpoints, record missing instrumentation, then apply only demonstrated general
   structural corrections. The typed dense-write proposal is conditional on data.

Public86 triage follows each numbered implementation step. After all three,
external/development/broad/sentinel must all improve; otherwise report and stop.
If they improve, run fresh complete qualification including cached/off exact
scans and immutable burndown rows, constant-cache fuzz, x86 RSS and final triage.
