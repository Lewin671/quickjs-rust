# Pre-existing binding findings repaired in round three

Each repair is a separate commit after the required S3b regression/stress fixes.
The lead confirmed identical baseline behavior; NG/spec are the behavior oracle.
Final qualification remains pending; no historical exact receipt proves these changes.

## Try/finally function declarations

Parent `d14c284a`. Try/finally lowering opened lexical scopes but omitted their
declaration-instantiation phase; FunctionDecl statement lowering itself only
leaves an empty completion. They now run the same predeclaration/hoisted-function
logic and Annex B blocking as regular blocks, preserving the existing protected
completion slots and abrupt-flow protocol. Focused tests cover calls before the
declaration, finally execution, sloppy var propagation and strict isolation.
Exact lead reproducer is a permanent NG fixture. The focused test failed before
repair and passes afterward; the staged fast gate supplies broader coverage.

## Captures outside inner block/catch shadows

Parent `62561f95`. Coarse AST function-local names included block/catch names,
so capture linking suppressed a name even where the compiled references were
unresolved outside that block. Capture demand now honors unresolved operations
in the compiled scope stream. NFE self names are passed explicitly as reserved
creation-scope bindings; they must never capture an enclosing homonym even when
an inner block also declares that name. Existing indexed routes/cells remain.

Before, reads threw and writes failed to reach the parent. Eight block/const/
for-let/catch read/write combinations now pass, plus a self-name precedence
control. The exact four-case lead reproducer is a permanent NG fixture.

## Default-parameter closure versus body declarations

Parent `34fbbc66`. Capture-demand repair exposed the outer cell, but the compiler
still used one lexical lookup layer for received parameter-scope captures and
body declarations. `var x` initialization selected the root capture and changed
it to inner (before: `inner:inner`, required: `outer:outer`). Function-body
compilation now opens a lookup layer aligned with its existing body descriptor
and registers own hoisted bindings there. Parameter closures already compiled
against the root layer keep their received-cell route. No per-call map is added.

Focused parameter suite passes 30 tests, including body var/let/const/function
homonyms and writes through the parameter closure. Exact lead case is an NG
fixture; existing block/self-scope controls also pass. The runtime representation
and ContainsExpression var-holder rule are unchanged.

## Inherited global lookup

Parent `72922bed`. VM bare/typeof fallback and wide global reads checked own
properties only. They now use HasProperty plus Get with globalThis as receiver,
including inherited data/getters; lexical/canonical cells still take precedence.
No inherited getter value is cached. Script, function, named-function and eval
reads each invoke the getter once, with correct receiver; typeof sees the data.

Because inherited hooks are newly reachable, loop entry/helper preparation now
requires a pure global read (receiver, canonical lexical, or ordinary own data).
An inherited/exotic route declines instead of probing/hoisting a getter. A
100-call numeric-helper test asserts 100 getter calls, preserving the P1 effect
boundary. Before repair the exact lead case threw ReferenceError; focused scope
suite and strengthened NG fixture cover both data and getter variants.

## With-reference receivers in tagged/optional calls

Parent `c7f8be93`. Ordinary calls retained the selected with-object receiver,
but tagged and optional forms compiled the identifier as a bare Value. The
compiler now shares one object-environment callee-reference lowering across
all three call forms. Resolve/Get occur once before argument/template effects;
optional short-circuit drops both receiver and nullish callee, skipping args.
Direct eval keeps its dedicated protocol.

Focused tests cover tagged/optional/spread calls, escaped with closures,
nullish short-circuit and RHS deletion of the selected property. The initial
case failed before repair; permanent NG fixture checks the same routes.

NG fixture note: pinned NG reports `InternalError: inconsistent stack size`
compiling a with-bound optional call in assignment/statement position. Returning
the same optional call from a nested function avoids its compiler defect while
preserving receiver/short-circuit observations. Candidate runtime tests retain
those valid assignment/statement programs; no behavior was changed toward NG.

## Labeled continue and TDZ on block re-entry

Parent `e194af4e`. Normal-exit ClearLocal instructions were skipped by labeled
continue, so an unexposed slot retained the previous iteration's initialized
value (`ReferenceError:1` instead of two ReferenceErrors). Block declaration
instantiation now clears its private lexicals at entry before references and
hoisted functions. Try/finally block entries use the same phase. This also
predeclares block-function locations before initializing any of them.

Clearing on the abrupt jump itself would precede finally reads, so that route
is deliberately unchanged. Focused tests cover for/while re-entry, escaped
initialized closures, escaped still-TDZ cells, and finally reading the exiting
instance. All eight scope-instantiation tests pass. Permanent NG fixture covers
the minimal re-entry and retained closure variants.
