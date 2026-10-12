# T039 activation component pricing at af4d0d3c

Fresh assays, 2026-10-12. Base e8b07b95 has identical runtime to af4d0d3c.
The production Rust executable SHA-256 is
6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b;
NG f7830186 executable is cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0.
Queue SHA-256 2882db837308dca8db304e3d0de0b32efcb4326332fe218393020c2f9e275f26.
[T039](../T039-tofte-activation-components.md) owns next action.

## Fixed activation, opened

Nonempty factories telescope: two parameters and a nonempty return with
unreachable direct eval; add the real binding set without initializers; add
all 28 original formatter declarations; add their six original arrays. Same
4/8 calls per outer iteration, N=1000, five warmed alternating engine pairs.
The Function.prototype.call wrapper is common to all stages. This is a
source-level joined decomposition, not exclusive machine-symbol accounting.

| Piece per activation | Cycles Rust / NG | Instructions Rust / NG |
| --- | ---: | ---: |
| Generic call/context, including potential arguments | 8398.61 / 1371.28 | 43213.32 / 8088.02 |
| Additional var binding/cell setup | 8643.63 / 184.42 | 45845.31 / 890.14 |
| 28 real declaration instantiations | 42160.44 / 24753.49 | 177324.21 / 150309.51 |
| Six arrays / 97 constant elements | 3349.48 / 3466.77 | 15322.51 / 20890.11 |
| Factory total | 62552.15 / 29775.96 | 281705.36 / 180177.80 |
| Original literal-format intercept (prior same-runtime assay) | 62430.54 / 30305.56 | 289521.55 / 186201.37 |
| Intercept minus factory transfer residual | -121.62 / 529.59 | 7816.19 / 6023.57 |

Adding actual split/join to the factory costs 4550.83 / 2351.67 cycles and
21361.24 / 14709.71 instructions. It cannot be added to the old intercept:
that would over-explain it. Different body temporaries, receiver/return work
and split/join lengths change the intercept. The transfer residual is kept
visible rather than called a removable stage. None of the factory assays
executes the unreachable eval; the fixed intercept excludes per-eval work.

Real case: 1000 formatDate activations, 28000 internal declarations, 45000
captured cell handles, 77055 callee-local classification visits (including
startup), 38000 freshly allocated VM cells (38/activation; the mapped
parameter cells already exist). Each activation overlays 40 live cells.
All 28000 closures bypass the same dynamic scope after the existing proof.
27000 memo hits still enumerate the parent locals. 1,400,000 of the measured
1,400,450 visits are closure scans; 450 are cold eval/full-view work.
Only 40360 overlay-cell hash events remain: 40000 first publication plus
360 from nine cold evals. This is **not** per-closure cloning of the scope.

Direct eval conservatively sets needs_arguments_object even when the source
contains no explicit arguments reference: a later unknown eval can read it.
function_env therefore builds the mapped object and canonical parameter cells.
It also makes every non-temporary local name-reachable through a cell.
Declaration instantiation captures only each child's actual cells plus one
shared DynamicBindings handle; it does not allocate a whole map per child.

## Scope/sibling scaling

Hold total parent slots fixed, change declarations 4/8 (also 24/28 near the
case), and price by the 8-minus-4 nonempty increment in both call and declaration
count. A includes entry/setup, B is effective marginal closure cost. N=1500.
No-eval functions still instantiate/escape a closure, so no empty-body tier
subtraction. Counter builds prove all expected NewFunction events occur.

| Scope slots | Direct eval present | B cycles R / NG | B instructions R / NG |
| ---: | --- | ---: | ---: |
| 16 | no | 587.36 / 768.26 | 3207.91 / 4805.49 |
| 50 | no | 614.85 / 701.39 | 3255.35 / 4781.00 |
| 100 | no | 613.05 / 773.92 | 3168.17 / 4820.51 |
| 16 | unreachable | 764.99 / 756.29 | 4079.14 / 4813.12 |
| 50 | unreachable | 1067.91 / 732.37 | 5308.66 / 4883.87 |
| 100 | unreachable | 1424.60 / 745.24 | 7477.98 / 4863.82 |
| 50; 24/28 siblings | no | 419.74 / 683.91 | 2907.30 / 4783.27 |
| 50; 24/28 siblings | unreachable | 1119.18 / 715.73 | 5101.77 / 4953.72 |

Rust eval-capable per-closure cost rises about 7.85 cycles / scope slot over
16->100; NG stays near 0.75k. No-eval Rust cost stays flat. H declarations in
S slots incur O(H*S) overlay validation, quadratic when H grows with S.
Sibling count changes do not cause an independent growing per-closure map
clone. The real-body 28-closure packet averages 1505.73 / 884.05 cycles,
6333.01 / 5368.20 instructions per closure; it includes different capture
counts and child metadata, so it is not substituted into the simple-sibling B.

## Direct eval, tiers and cache

There are 5500 evals: **5.5/formatDate activation, 11/outer iteration**, not
11 per activation. Nine distinct tiny sources compile once on Rust; 5491
hits use immutable Rc bytecode. No AST is retained. The per-realm FIFO cache
has 32 entries and a 4096-byte insertion bound, keyed by source + EvalParseContext
(strict, function/method/derived/field context, visible private names).
A caller scope-layout key is unnecessary for these blueprints: free names
remain runtime lookups. Values, shadowing and eval-added bindings are not baked
into bytecode. Declaration/function/template identity-bearing sources retain
fresh compilation; with/nested eval and parameter contexts retain conservative
projection/instantiation. Adding the proposed cache is redundant for this case.

NG __JS_EvalInternal reparses and creates bytecode on every direct eval;
add_closure_variables imports enclosing descriptors before parsing. Its parser
emits intermediate code while parsing; “parse” and initial emission are not
independent stages. js_create_function resolves/optimizes/finalizes afterwards.
Standalone normal release reference assays call these original functions;
no engine timers/counters/source edits are used. Rust frontend assays use
public lexer/parser/compiler APIs with the workspace-pinned dependency lock.
Frontend/runtime phase prices and source/context limitations are retained below
when the assays complete. Execution/closure destruction that share one public
API are reported together, never split using inclusive sampler percentages.

Warm eval-call B at 40 extra locals: 7286.61 / 11065.41 cycles,
38810.10 / 67445.71 instructions, including a captured wide getTime callee.
The same direct call in an eval-capable frame is priced separately; scope-size
changes also alter NG compilation. Cold/warm comment-source assays initially
reused the same source within one outer iteration and are not exclusive cache
miss prices; do not derive compilation savings from their difference.
Real Rust projection visits 549100 slots for 5491 hits (50 twice); full caller
views occur only 10 times including startup. Native split/join already get a
realm-only context in vm_call.rs. No repeated native caller snapshot remains.

formatDate stays generic with and without eval because it creates capturing
closures; removing eval alone does not admit the whole body to wide. Its inner
formatters are standalone wide while their scope bypass stays valid. Uncaptured
locals in the non-eval frame can be authoritative; eval makes all non-temporary
bindings cells, not only the nine currently selected names (the source is
runtime data). Same nonempty interpreted load/add/store group: 69.45 / 20.34
cycles without eval, 166.23 / 14.89 with unreachable eval; 404.97 / 35.08 vs
960.97 / 36.90 instructions. The catch boundary keeps Rust on the interpreter
in both. It is a group price, not a single-load price or a transferable exact
cost for all binding opcodes. Observable arguments/TDZ/module paths remain.

## Residual and leaf ranking

The old 23.01% cycle-gap residual was 13.82M cycles, not 23% of Rust case.
Generic local-access group penalty has reach on 118002 slow loads, 37000 slow
stores and 11000 AssignLocal operations, but their mix differs from the group;
no independent per-op partition is asserted. The split/join + real number/string
result mix and intercept transfer error account for work omitted by constant
character probes. Date UTC/local prices remain excluded as an identical-work
claim. String conversion/wrapper probes were already cheaper on Rust; no new
conversion cache or benchmark-specific formatting shortcut is justified.
String search admission analogue was 7.51% case including entry; typed_loop
is now excluded. A native borrowed scan's 15.88% ideal ceiling is still not a
proved general loop lowering/continuation unit.

| Candidate | Reach / CASE | Decision |
| --- | --- | --- |
| Fixed-scope binding-instantiation metadata | Overlay inclusive 9.02% (27/28 scans removable), separate pure resolver 2.18%, plus private-cell realm classification | Select leaf, target <=0.92; no activation ABI migration |
| Compilation cache source/layout/strictness | Only nine Rust cold compiles | Existing cache; new key cannot pay 8% |
| Selected-name env projection | 5.48% inclusive, already prunes values but scans slots | Below 8% alone; future leaf, not this plan |
| Narrow local/cell access lowering | Interpreter group penalty, mixed slow-op count | Scalar/cell effects need dedicated lower-level pricing; no proven 8% packet |
| Lazy/batched hoisted functions | 28 instantiations, only some functions invoked | Current eval can observe any sibling, closures escape/retain cells; a lazy canonical binding state must preserve identity, capture time, mapped arguments/private/name contexts. No sound discard/batch admission for current dynamic source; not selected |

July/September incremental overlay failures retained active-cell snapshots
and the full validation walk. This plan removes that walk only under immutable
binding-kind/effect proof; map identity/generation and exact first/remap overlay
remain. It also caches immutable captured-slot classification instead of
allocating unused child-source plans each activation. It never matches the
source/bytecode template or names of arrayExists/formatDate, any token/date,
array literal contents, number of siblings, scope size or benchmark identity.

## Provenance and verification

All measurements/builds/counter runs use lockf /tmp/qjs-lead/host-timing.lock;
five warmed pairs/cell and quiet build/engine checks. Base executable reused
verbatim, no formal perf-compare. Scratch target/activation-pricing raw JSON is
local/unretained; the prices, counts and reproducible assay sources are here.
The exact-base receipt copied from the prior same-runtime assay has SHA-256
888b37ac6a4acfda919c10b05a4b272c50dff00551578c98211f9ad8088963d9.
Frozen plan validate-unit passed against the af4d0d3c queue and that receipt.
No temporary counter edit remains in crates before the plan commit.

## Frontend/runtime phase assays

For the same nonempty g()-sized source, Rust normal release API harness prices
are lexer 541.43 cycles / 3431.38 instructions, lexer+parser 2242.79 / 11556.68,
compiler on retained AST 3596.42 / 19887.01. Nine cold compilations cost roughly
53k cycles per whole case (<0.03%); 5491 hits do zero lex/parse/compile work.
NG contextual assays (40 dummy scope bindings, same direct parse context):

| Phase increment / eval | Cycles | Instructions |
| --- | ---: | ---: |
| Function-definition/context/closure-descriptor setup + destruction | 986.80 | 5895.06 |
| Lexer beyond context | 92.61 | 557.73 |
| Parse/intermediate emission beyond lexer | 1509.16 | 9395.54 |
| Resolve/optimize/finalize beyond parse | 2334.17 | 13279.11 |
| Context+frontend total (includes final blueprint destruction) | 4922.75 | 29127.44 |
| Cached blueprint js_closure binding + closure destruction | 1147.28 | 7057.68 |
| Execution, nested nonempty call and frame return/drop | 84.20 | 531.38 |
| Full cached execution: binding + execution + destruction | 1184.93 | 7597.14 |

The last three assays bind against a real running JS parent frame. Full differs
from the independent component sum by -46.55 cycles / 8.07 instructions:
these are effective release increments, not a closed exclusive cycle model.
Allocation/destruction are paired by the APIs; no separate destructor-only
cost is identifiable with whole-process counters. That work is included and
priced in the scope/destruction and execution/frame-drop rows, not assigned
an invented standalone value. NG parse and intermediate emission likewise
remain one stage. Rust's warm direct eval scope/protocol/VM total is its joined
7286.61-cycle call envelope; the callee/control and frontend prices constrain it,
but a machine-symbol split of setup versus VM/drop is not established.
The public global-eval reference assay independently gives compile 4512.45,
full 5109.53, reusable bytecode 554.05 cycles; differing context prevents
substituting those numbers for the real 40-binding direct-eval total.

Initial diagnostic counter compilation failed with
`error[E0658]: attributes on expressions are experimental` for macro match
arms; braced statements fixed it. Failed builds were never timed. Initial
frontend helpers used g()+0 on Rust/public NG versus g() on the contextual
reference; those inconsistent prices were discarded and matching g() assays
rerun. Rust helper dependencies were pinned by copying workspace Cargo.lock.
Neither reference source nor workspace Cargo files were edited.
