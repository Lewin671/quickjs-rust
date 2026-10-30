# T077 S3: T074 decisions for Context and activations

Approved by the lead 2026-10-30: all six decisions below as proposed; not implemented. S3b freezes the verification matrix. Source audit: `dd0905b2`, 2026-10-29.
[T074](../T074-direct-eval-scope-model.md) owns the known gaps; the
[stage plan](T077-s3-plan.md) owns scheduling. These decisions are needed
before S3b; S3a may bind native entries without changing eval semantics.

## Proposed binding and scope contract

1. Keep positional locals and received shared cells. A `BindingRef` is a typed
   route plus immutable binding metadata: mutable, initialized/TDZ policy,
   strict-write policy, deletable and binding kind. Shared cells hold the sole
   current value and initialization state; metadata cannot be discarded when
   an eval name resolves to that cell. `Init` differs from `Assign`.
2. `ScopeDescriptor` is immutable compile-time data: lexical declaration order,
   names, BindingIds, parent boundary, parameter/body scopes and lookup routes.
   It contains no live values. `ScopeLease` owns parent leases, promoted cells,
   optional object environments and an optional **own** extensible var holder.
   No lease points at a pooled activation or recyclable register block.
3. Ordinary functions get fresh variable-environment identity. Sloppy direct
   eval inserts var/function declarations into the caller's variable environment,
   even if an ancestor has the same name. Existing own bindings reuse their
   cells. Parent links stay live: never fork/copy the inherited name map at call
   entry. Strict eval gets its own variable/declarative environments; indirect
   eval uses the selected realm, without caller lexical visibility.
4. A runtime-created binding lives in an own name-to-cell table with declaration
   attributes; parent lookup is a chain walk. Deletion removes the own entry if
   deletable and affects later resolution, never a cached copy of its value.
   Static declared var/function bindings are not made deletable by later eval.
   References already resolved retain their selected base/cell until consumed.
5. Compile an `EvalExposurePlan`: a closure whose lexical descendants may use
   direct eval retains descriptors and all potentially visible bindings, not
   merely static free names. Promote locals into their unique shared cells at
   exposure, redirect all reads/writes, and clear their slot ownership. Include
   uninitialized lexicals and subsequent lexical declarations with TDZ state.
   No per-call name-to-value table. Unexposed functions retain slot-only access.
6. Names crossing a variable environment that sloppy eval may extend use a
   live resolution route through that environment, even when the child has no
   eval site. A later declaration can shadow a farther parent binding. Statically
   resolved names nearer than that environment keep direct slot/cell routes.
   Cache only resolved routes with structural invalidation, never binding values.
7. Declarative scope records, parameter/body splits, catch scopes and `with`
   object scopes form one ordered chain. `with` consults HasProperty and
   unscopables in specification order, allowing callbacks. Declaration
   instantiation chooses the variable environment and checks lexical conflicts;
   the initializer's identifier assignment resolves independently through the
   actual object/declarative chain. Capture the reference before RHS effects.
8. Typed lexical-call context is separate from name resolution: realm, this /
   derived-this cell, new.target owner, home/super, private environment, active
   constructor, field-initializer restrictions and ScriptOrModule/referrer.
   `arguments` remains a lexical binding with an explicit Absent/Present/TDZ
   distinction. No internal context slot can shadow a later lexical declaration.
9. For sloppy simple parameters, mapped arguments indices alias the exact
   parameter cells, with duplicate-name and descriptor/delete severing rules.
   Materialize the arguments object when observable; eval exposure can demand
   it even without a static `arguments` read. Strict/non-simple parameters use
   unmapped arguments. Retain actual argc/extras only while semantically needed.
10. Suspensions own the same leases/cells/context and pending semantic step.
    Computed class keys, static blocks and field initialization retain the
    enclosing activation's yield/await continuation, not an ordinary thunk call.
    A module lexical receiver is initialized undefined; derived-this TDZ is a
    different state. Arrows inherit the lexical owner and scope chain.

## Decisions requiring lead architecture approval

| Decision | Proposed choice | Consequence / alternative cost |
| --- | --- | --- |
| Exposure boundary | Transitive syntactic potential-direct-eval analysis plus lazy cell promotion | Retaining static-free names only is incorrect; promoting every local on every call adds avoidable cost. Freeze exact propagation before code. |
| Extensible variable environment | Fresh own holder, live parent chain, typed declaration attributes | Parent-map fork loses later additions; inserting into a found outer cell violates declaration ownership. |
| Dynamic routes through extensible scopes | Late lookup or invalidated route cache for crossing names | Static captured farther-cell shortcuts cannot survive intervening eval shadowing. |
| Binding metadata | One cell initialization authority plus immutable declaration metadata referenced by every route | Merely preserving `Rc<RefCell<Value>>` identity does not enforce const/TDZ/deletion. Decide exact private type before Context port. |
| Context/name separation | `arguments` is lexical; other call context uses typed owner fields | Fixes absence versus TDZ and module versus derived receiver; requires all constructor/arrow/deferred-key users to migrate together. |
| Landing contract | Fix all T074 discriminators as part of S3b, before activation reuse | S3b is a semantic repair as well as representation change. Do not bless baseline wrong results as equivalence. |

No new public embedding API, dependency, GC or second environment authority is
proposed. This is an internal contract approval, not permission to change the
protected model. Update the owning env/suspension design notes only when these
semantics actually land; this proposal must not describe them as present today.

## Verification matrix (freeze before S3b implementation)

Use the exact independent snippets in T074, with NG output as the discriminator;
copy none of its baseline wrong answers into expected results. Add lowest-layer
route/lease tests and permanent runtime/NG fixtures for every row below.

| T074 item | Required result / identity proof |
| --- | --- |
| 1 eval-only capture | `outer(8)()` returns 8 after outer return; same cell for ordinary sibling writes. |
| 2 live inherited declarations | Var/function additions after closure entry remain visible; generator and nested rebinding return 5 / `[2,2]`; holder identity unchanged. |
| 3 const and TDZ | Outer const write throws TypeError; uninitialized lexical throws ReferenceError and remains uninitialized; no successful write-back. |
| 4 module deferred key | Private brand plus `String(this)` completes with module this undefined; no fake derived-this error; preserve global receiver and arguments absence. |
| 5 own eval declarations | Inner/outer var and function return `[2,1]`; include parameters, preexisting closures, prior reads and strict eval. |
| 6 deletion | Eval-created var deletes successfully and later typeof is undefined; static var cannot delete; parent binding revealed after own deletion. |
| 7 mapped arguments | Eval assignment to `arguments[0]` updates parameter to 2; reverse write, duplicate params and mapping severance covered. |
| 8 with initializer | `[1,2]`, not `[2,0]`; Proxy/unscopables mutation verifies order, once-only reference resolution and failure behavior. |
| 9 same-frame const | TypeError for eval assignment; identical check whether reached by static route or dynamic resolution. |
| 10 deferred-key suspension | Generator yields key then returns function; async prints function; pending finally/private/derived context survives. |
| 11 lexical arguments shadow | Arrow sees 3 after initialization; ReferenceError before, regardless of implicit call-context availability. |
| 12 nested global arrows | typeof absent arguments is undefined; missing lexical owner is not TDZ; include parameter-eval declarations. |
| 13 exited block shadow | Later direct eval sees outer arguments object; no stale block cell wins. |

Also retain every T071 regression fixture identified by
[T071 review verification](../archive/T071-review-verification.md),
[narrowed evidence](../archive/T071-dd312dac-verification.md), and
[flat iterations](../archive/T071-flat-correctness-iterations.md): ordinary
parameter defaults, rest/arguments, named self/class-name immutability, class-key
private/this/new.target/super, field initializer and module context. Audit the
actual test inventory at the implementation base; these historical branches are
constraints, never code to cherry-pick blindly.

Run capture promotion and frame-pool poisoning modes, early/late eval and
callback writes, strict/sloppy and cross-realm variants, block/loop lifetime,
catch/finally, import live cells, suspended generator/async/TLA and agent context.
Then the common full/NG/complete exact gates from T077, with zero lost passes or
new exclusions. The current configured zero-gap standing does not cover all
known T074 reproducers; this proposal makes them explicit additional obligations.
