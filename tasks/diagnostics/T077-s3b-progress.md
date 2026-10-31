# T077 S3b implementation record

Implementation base: `1e6e2a2b8ccdb590d735e5b6e969105cb1344c72`, accepted
S1+S2 integration (runtime `dc710e4b`, acceptance `fc3deef7`). Frozen brief:
`71294a20864662808216f874635dc8fb0473693e`. Owner branch:
`agent/t077-s3b-context/codex`. User authorizes local commits only, no push/merge.
The integrated S2 semantic API is the input; Context does not access NamedStore
internals. Prototype/layout follow-up paths remain outside this owner boundary.

Predecessor performance endpoint is this accepted S1+S2 base; cumulative endpoint
is `6663738f` per T077. Lead owns exact timing binaries/corpus/recipe pinning and
formal/held-out/RSS acceptance; owner collects public86 quick triage via the iMac
queue only. No performance standing is claimed by this record.

## Pre-code inventory recount

The frozen regex over runtime Rust sources counts **198 files / 2,122 tokens**
for CallEnv at the base, versus 196 / 2,112 at design. Delta by family:
array 18/179 (+1 token), bytecode 32/208 (+1 file/+6 tokens), function 10/92
(-1 token), global.rs 1/46 (+1 token), object 11/94 (+1 file/+3 tokens).
All other families are unchanged. The 98 inherent methods remain (including
one test-only cache query); Clone/Debug remain separate. S3a native bind arm
surface is unchanged. Migration/deletion authority remains the frozen brief.

## Global accessor binding repair

Base reproducer `/tmp/qjs-s2r/t/glob6.js`: `a 1 1`, `b 1 3`, `c 1/1 4`,
`d 1 5`, `e 1 6`, `top 1 7`. The setter ran, but apply-back incorrectly
published its assigned value as a realm data binding. The sync helper now
checks the current own descriptor and invalidates the realm data route for an
accessor without invoking the getter/setter again. Audit also found interpreter
`set_property_value` and compact `property::set_named` publishing the assigned
value after successful [[Set]], including setter success. Both now inspect the
resulting own descriptor (setter may redefine/delete it), publish only its data
value and invalidate otherwise. This is an additional repair site within the
brief's binding-authority bug, not an architecture deviation.
Permanent fixture checks alias/globalThis/identifier writes and exact getter
count. Lowest-layer runtime regression and all **269 NG fixtures** pass on the
working tree. Staged fast gate passes (including 65 selected Test262 cases),
combined agents/perf-counters CLI compile passes; exact variable-statement slice
passes 178/178 with zero gaps. This filtered scan is not a complete-suite scan
and does not create a burndown entry. No complete correctness or timing standing
is claimed for this preliminary repair.

## B1 binding-plan migration

BindingPlan replaces FrameInitPlan. The existing compiler records typed scope
IDs and declarations (parameter/body/block/catch/loop/class-name/with), including
future TDZ declarations in active scope descriptors. Exposure sites retain only
their active scope IDs. Immutable metadata contains declaration kind, write/init
policy and eval deletability; B2/B3 connect these to live cells and leases.
Capture routes are linked at body compilation for functions and class families,
with received order preserved and parent locals/upvalues addressed positionally.
Mapped-arguments duplicate selection is compiled once instead of scanning the
parameter AST on every arguments-object creation. Raw IR constructors are now
test-only; production bodies consume compiler scope metadata in one preparation.
No runtime ABI switch, CallEnv deletion or repaired T074 semantics is claimed
at B1. Six plan/descriptor tests pass; default/agents/perf-counters compilation and
269 NG fixtures pass. The fast gate exposed an old raw-IR test whose explicit
parameter slots were out of bounds of its empty locals; the test now supplies
valid parameter slots while preserving its duplicate-position assertion.
Fast gate passes at `84f3e7ef` (2,502 runtime tests and selected Test262
slices). B1 is present; this establishes no S3b exit or timing acceptance.

## B2a canonical cells and declaration policies

BindingCell replaces the old Upvalue type. Cells retain the immutable compiled
metadata owner and explicit Uninitialized/Initialized state. VM declarative
stores and dynamic assignment selection enforce Init/Assign and const/TDZ;
mapped parameter accessors use the parameter's exact cell and checked Assign.
Eval demand now requests arguments even without a static arguments read.
Compiler-owned cells are excluded from compatibility apply-back: a setter's
canonical write must not be overwritten by an older service-view projection.
The permanent fixture catches both equal-value const assignment and this
mapped-arguments stale publication.

Global lexicals share canonical metadata-bearing realm cells. Deleted are the
copied global lexical value projection, global lexical/mutability side sets and
their redundant realm-scope wrapper. Global properties of the same name remain
independent. Root scope descriptors distinguish real top-level declarations
from loop/switch/catch/class-name bindings; evalScript explicitly selects the
persistent lexical routes. Simple parameters share the variable target with
body vars; non-simple parameters retain a separate body target.

Migration correction: B2's complete raw-writer deletion is coupled to B3's
removal of compatibility projections/apply-back. B2a establishes metadata and
canonical cells while retaining the old service-view publisher temporarily;
the remaining B2/B3 cut removes that publisher together with the views, rather
than adding a second checked/unchecked lexical authority. No B2 completion is
claimed yet. B3 must replace flat deopt maps with leases and own var holders,
remove all raw BindingCell::set calls, and implement the rest of T074.

Verification at `ac914ebb`: staged fast gate, default/agents/perf-counters
compile, full check (including 5,170 curated cases), 2,507 runtime tests and
270 NG fixtures pass. Complete exact cached scan: Rust 42,672/42,672,
zero fail/timeout/not-run; NG 42,602 passes/70 failures; zero gaps. Total records
53,572, unchanged skips 10,900. The full case map hash is identical to the
accepted S2 map: `2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.
Burndown is appended at the verified runtime SHA, not the current B3 draft.
Production CLI was built with `cargo build -p qjs-cli --release --features
agents --target-dir target/s3b-b2a-production`; agent-ready smoke succeeds.
CLI SHA-256: `7341b7821c963ffa20f769a2e4ffa55e16828fa2f1290d743c86ee45cca8bc93`.
Summary hash: `cffdf394963ec66c41d006192593e9975e6b082a55483748550b7a9fb866e80f`.
Command: pinned QJS_CLI_BIN, QJS_AGENTS=1, `find-qjsng-gaps.sh --exact --all
--no-recommend --out-dir target/s3b-b2a-exact`. Input Test262 pin is `42496613`.
IC-off, stress, counters and timing remain final B6 obligations; this is not S3b
exit or performance acceptance.

## B3a live binding authority

The owner implements descriptor/cell leases, fresh own var holders, live parent
links, explicit eval entry roots and global binding targets. Parameter/body
locations split for non-simple lists, including destructured leaves and TDZ;
fresh loop/catch cells preserve metadata. Saved with references survive RHS
suspension. Scope instances/holders/references are retained by snapshots.
Functions retain optional escaping scope in existing cold state.

Deleted: selected_direct_eval_call_env/cached_direct_eval_call_env and their
selected-name scan, writes_bindings apply-back proof, Host evalScript value
write-back, and the unused completion sloppy-name projection. Compiled eval
plans still cache; each invocation resolves its own live chain.

Source audit found that legacy frame initialization replaced selected eval cells
with new cells; preserving those routes fixes two-location writes and eval loop
nontermination. Global eval and host script need an explicit global target even
though their IR is function-shaped; otherwise declarations alias private frames
or publish into the caller realm. Temporary typed entry/global-selection fields
on CallEnv bridge to B4's CreationContext/ActivationContext, without another live
value table. Host script function descriptor attributes remain distinct from
sloppy global eval. Selected foreign globals use live object semantics and never
cache/publish their values in the caller's realm. Native setters retain getter
authority, including lexical/property homonyms.

Verification on this code: 2,512 runtime tests and 271 NG fixtures pass; 15
permanent non-key/parameter T074 discriminators match NG; new lowest-layer
holder/lease/selected-cell tests pass. Default/agents/perf-counters compile and
Clippy pass. Staged gate passes at `ea53dfa7` including 117 selected Test262 cases. No S3b exit/performance acceptance.
Remaining B3 policy/reference audit and compatibility deletion join B4/B5:
class-key suspension/private/lexical owner, named self, complete Init/Assign and
saved crossing-name references, borrowed Context/native ABI and all raw view
retirement. Final stress/exact cached+off/counters/triage remain B6 obligations.

## B4a enclosing class-key continuations

Computed keys, including private-name keys, now lower into the enclosing IR
between NewClass/BeginClass/DefineClassElement/FinishClass. No deferred key
function, expression classifier or nested-key capture traversal remains.
Cold ClassKeyContinuation retains partial constructor/prototype, the canonical
inner-name cell, source-ordered pending statics and private overlay; VM PC,
operand stack and scope instances supply the enclosing continuation state.
Snapshots move the region; try handlers restore their class-region depth.
This is a representation correction to the frozen continuation payload: existing
VM continuation fields are not duplicated inside each class region.

Heritage resolves before keys; each method installs before later key effects.
The inner-name cell is the declaring slot's cell, TDZ throughout keys and
initialized before static initialization. Inferred display names create no
lexical binding. Private methods install before static initialization, then
public/private fields and blocks run in one source order. There is no extra
constructor-owned reference to a class cell merely to build its members.

Recovered all seven T071 script/module fixtures and all three historical direct
eval test modules. Permissive const/TDZ checks now demand the specified errors.
Recovery exposed two B3 route bugs: a received global lexical needed live lookup
when its own function contains eval, even without a retained parent lease; and
catch cleanup inferred scope ownership from local allocation order, clearing an
implicit arguments cell discovered inside try. Cleanup now uses declaration
scope IDs. Module roots explicitly seed initialized undefined this, and eval,
module, field and static-block roots do not invent implicit arguments owners.

Verification on this working tree: 2,540 runtime tests and 279 NG script fixtures
pass; Clippy/workspace/all-targets and agents/perf-counters compilation pass.
Separate module comparison: `TZ=UTC target/debug/qjs --module
 tests/fixtures/compare-modules/deferred-key-arguments.mjs` and
`TZ=UTC third_party/quickjs-ng/build/qjs -m` for the same file both exit 0 and
print `undefined:undefined:undefined:undefined` then `function:function`.
Module fixture SHA-256:
`44d067f0257e9b40cef5a3838377079788427c86466992e22bea359fa9d6a499`.
Staged gate passes at `ecdacbda`, including 65 selected Test262 cases. No complete
exact scan at this code, no final S3b exit or performance standing.

## B2b self-name and import policies

Named expressions compile one immutable name plan. Creation initializes its
cell once and retains its lease outside parameter/body environments; call entry
no longer inserts a name/value copy. Deleted immutable_name_caller_value and
apply_completion_to_caller, the saved caller binding and shadow-name body scan.
A callback's legitimate write to a caller homonym now survives return/throw.
WritePolicy distinguishes IgnoreSloppy from always-throw const/class bindings;
class key regions supply strictness for surrounding sloppy bodies. Import
references reject writes before reading exporter TDZ. Function constructors'
`anonymous` display name no longer invents a lexical self binding.

Oracle audit (NG `f7830186`, Node v22.23.3): new runtime self-name tests follow
[ES2025 named expressions](https://tc39.es/ecma262/2025/multipage/ecmascript-language-functions-and-classes.html#sec-runtime-semantics-instantiateordinaryfunctionexpression),
[SetMutableBinding](https://tc39.es/ecma262/2025/multipage/executable-code-and-execution-contexts.html#sec-declarative-environment-records-setmutablebinding-n-v-s),
and [CreateDynamicFunction](https://tc39.es/ecma262/2025/multipage/fundamental-objects.html#sec-createdynamicfunction).
Observed independent scripts, run with Rust `--raw -e`, NG `--script -e
'console.log(expr)'`, and Node `-e 'console.log(expr)'`:

| Runtime test in direct_eval/self_binding.rs | Rust / Node | NG |
| --- | --- | --- |
| strict eval self assignment | TypeError caught = true | false |
| strict class-key self assignment in sloppy function | TypeError caught = true | false |
| own eval var shadows self for an earlier arrow | number:8 | function:self source |
| Function body typeof anonymous in fresh realm | undefined | function |

These NG differences are recorded, not copied into runtime expectations. The new
NG script fixture covers caller homonym writes, sloppy immutable writes and
parameter shadowing where the independent oracle agrees. No Test262 exclusion
or expected-failure change is made. Full runtime tests pass (2,546 before the
last identity test); all five lowest-layer scope tests and 280 NG fixtures pass.
Agents/perf-counters compilation and Clippy pass. Staged gate passes at `e9c4e4e1`, including 117 selected Test262 cases.

## B4b typed lexical call owners

CallOwner/ThisBinding/LexicalCallContext now own receiver, lexical target,
home/super, active constructor and script/module/function/field/static kind.
Retention moves Inline into one shared Rc and redirects the original. Arrows
retain that owner in Function cold state only when their compiled operations or
lexical descendants demand it; parameter-only arrows allocate no lexical owner
or cold lexical state. A generator-key identity test proves the same owner is
retained by arrows before/after yield, with the original receiver. Snapshots
retain the owner through their existing environment handle; no owner is reset
on resume.

Deleted: receiver capture-slot synthesis and forced-this capture scan, Bytecode
this_slot and its initialization/super mirrors, receiver entries in root realm
maps, derived-this-special value forks, captured new.target cells and the extra
lexical-derived-this pair. Removed all receiver/home/super/active-constructor/
field/function-context/new-target marker constants and their runtime consumers.
The remaining keyword-shaped IR load selects the typed receiver directly.
Native construct dispatch uses a separate typed pending target until B5's borrowed
NativeSite; it cannot replace lexical new.target. Direct constructor shortcuts
now initialize that owner at entry. Fields/static blocks set their own kind;
eval restrictions follow arrows and stop at ordinary functions. Script root
receiver selection uses the global object, preserving it when globalThis.this
is written. Module entry initializes undefined, without an implicit binding cell.

This is a B4 sub-cut, not the Context ABI switch: CreationContext/ActivationContext
wrapping, generic binding reference selection and the final compatibility deletion
remain. Verification on this working tree: 2,554 runtime tests, 282 NG script
fixtures, Clippy/workspace/all-targets and agents/perf-counters compile pass.
The recovered module still exits 0 and matches both output lines against NG with
TZ=UTC. Staged gate pending. No S3b exit or performance standing is claimed.
