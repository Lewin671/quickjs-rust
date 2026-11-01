# Round-three binding repairs

## R3: self-name scope and positional captures

Parent `4853bd7f`. Before repair, the strict/plain/generator/callback discriminator
returned `global:global:global:3:0:global:3`; required
`param:param:param:3:3:global:0`. Dynamic crossing through a retained self-name
scope found no enclosing lease and selected a global before the already-received
cell. Reads and saved assignment references now fall back to that positional
cell after live crossing fails, ahead of module/global homonyms. Import policy
is retained only when the cell is actually the import's cell.

Validation: focused discriminator and all 68 direct-eval tests pass; exact lead
reproducer matches NG (`A param|B let|C global=0|D 1 global=g|E local`). Staged fast
gate passes 2,673 runtime tests and 65 selected Test262 cases. No full or exact
standing is claimed yet. Standalone R2 commit remains unchanged.

## F4: ContainsExpression, not IsSimpleParameterList

Parent `07364dab`. Compiler used non-simple parameters for both argument
instantiation and var-environment splitting. Rest/bare destructuring require
special instantiation but share the parameter variable holder. A recursive
ContainsExpression check now splits only for defaults/computed keys (including
nested/rest patterns). The existing unmapped-arguments rule is unchanged.

Before: direct eval `var c` replaced a rest/destructured parameter with undefined.
After: focused parameter suite passes; plan tests assert boundary 0 for four
expression-free forms and boundary 1 for five expression forms. NG fixture
covers the exact lead rest/object reproducer. Expression-parameter assertions
follow the spec/lead result and deliberately do not copy NG's known difference.

## F5: canonical metadata versus cache eligibility

Parent `beda2a29`. `is_realm_binding_cell` intentionally excludes declarative
lexicals because its production callers prove object-cache eligibility. The
verifier used that predicate to identify external canonical metadata, comparing
an eval-created free route's mutable placeholder to the actual const's Throw
policy. The exact lead case panicked before repair. Verification now checks
actual realm cell identity, including lexical cells, rather than IC eligibility;
slot/cell exclusivity and received-order checks remain active.

Focused test creates an explicitly verified/poisoned RuntimeServices without
process environment mutation. It checks eval-created function and block-shadow
routes: TypeError and unchanged const. These shapes were absent from the old
stress inventory. Passing that fixed inventory did not establish all valid
binding programs; round 3 adds the reproducers and seeded cross-mode fuzzing.
