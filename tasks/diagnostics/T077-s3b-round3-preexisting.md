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
