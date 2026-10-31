# S2 complete typed indexed-write routing

Before implementation at a8e7709f. Non-timing job
`20261031T010922Z-t077-s2-r2-lazy-array-instructions` measures nsieve 1.082005
main instructions, improved from e0c16e96 1.099518. The common target-specific
own dense overwrite is wired into interpreter/compact writes. Typed loops still
try a whole-array dense lease, which declines for unrelated holes, then go
straight to prototype checks and general ArrayRef::set. Thus their cold-named
hole fallback misses the newly verified own-element API.

Try that same API in both scalar/boxed typed fallbacks after numeric index
validation, before prototype checks. No new proof, cache, authority or admission
heuristic; all checks remain in ArrayRef. Holes/growth/readonly descriptors still
use the unchanged fallback. Verify mixed holes, accessors and sealed/frozen
arrays in typed execution, focused/full/debug gates and fresh same-case cohorts.
No dependency, unsafe, binding/native-entry API change or held-out work.
