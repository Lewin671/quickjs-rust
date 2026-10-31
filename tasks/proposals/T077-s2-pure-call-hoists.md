# S2 entry-proved native-call effects for invariant reads

Freeze before implementation at f5a83119. Canonical non-timing job
`20261030T215045Z-t077-s2-r2-slot-load-instructions-r2` still has string_slice
1.065996 main instructions after live-slot copy refinement. The earlier profile
`20261030T185822Z-t077-s2-r2-primitive-profiles` attributes 10.46% candidate self
samples to read_object. The typed loop currently blocks invariant reads whenever
CallClosedFormLeaf occurs, including primitive String operations with scalar
arguments that cannot run JavaScript or mutate named properties.

Permit an optional invariant-read variant only when every such call has scalar
arguments and its callee is traceable through copies to an entry-invariant
register. Track register provenance through straight-line blocks, clearing
non-invariant aliases at joins. Pin all hoisted/register guard inputs. At entry,
read only pure data properties; then prove every live callee is an explicitly
side-effect-free Math numeric native, or a String sequence/index native with an
invariant primitive String receiver. Restrict by operation/type/effects, never
by source, name spelling or opcode template. No getters/coercion callbacks.
Existing unknown stores, pushes and computed writes continue blocking hoisting;
same-name named overwrites continue excluding that read.

Compile the normal region independently as the unhoisted variant. If any entry
read or call effect proof fails before effects, use that normal region with its
existing guards/deoptimization. User helpers keep their original admission.
This is one register executor with two compile-time operation lists, not an old
property store or a second semantic authority. Hoisted values live only in the
activation's registers, are cleared on exit, and never enter shape IC metadata.
There is no cached receiver, function Value or method identity across entries.

Test native replacements between entries, user methods mutating prototypes,
getter counts, nonprimitive receivers/coercion, branch provenance and changed
value types. Verify the compiler actually emits the guarded variant for admitted
operations. Re-run focused tiers, full gates, canonical public counts/triage and
exact scans for retained source. No unsafe, dependency, binding/S3 API change,
formal or held-out run. Reject if general controls regress overall payoff.
