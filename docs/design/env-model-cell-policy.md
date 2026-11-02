# Compiled captured-write policy

`bytecode/binding_writes.rs` propagates compiler proof for non-global, non-dynamic
var and simple-parameter declarations. ParentLocal/ParentUpvalue preserve that
proof; Named routes do not. Closure linking validates declaration provenance,
name, mutable kind and completed initialization against the actual retained cell.
Observing an initialized value alone never supplies the proof.

Wide captured loads/stores use compiled received indices; general frames link
slot capabilities at preparation, completed parameters and promotion. The body
owns a bounded 128-capture proof mask. A closure enables it only when linking
validates every indicated retained cell; a failed link keeps all writes checked.
One instance bit selects that immutable mask without allocating another owner.
InitializedMutable refines Mutable metadata without changing assignment semantics.
Absent/out-of-range routes retain checked Assign. The mask owns no values and
does not replace cell identity.
Merely initialized lexicals, imports, NFE names, object-route cells, default-
parameter TDZ and extensible dynamic lookup never acquire the capability.

A proved store uses one safe RefCell replacement without release-build metadata,
TDZ or strictness discovery. Debug validates at link/store. Scalar ownership tests
avoid generic drop glue; resource-owning values are published before the previous
resource is released, and no storage borrow survives resource teardown/callbacks.
Initialization remains distinct from assignment; all unproven stores keep the
checked operation. See [binding authority](env-model-rewrite.md).

Own non-global static Var/Lexical/Catch/Function reads use a compiled route mask
and the current indexed cell, preserving TDZ and Value ownership. Compatibility
`from_env` does not identify received cells: use declaration kind and the received
classification. Eval-source/deletable bindings, parameters/arguments joins,
imports/received names and selected global roots retain semantic resolution.
Scope entry/release replaces or clears the indexed handle, never an escaped cell.
