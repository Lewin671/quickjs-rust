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
