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
TZ=UTC. Staged gate passes at `3fe0b5b2`, including 117 selected Test262 cases. No S3b exit or performance standing is claimed.

### Complete B4b checkpoint and with fallback repair

The immutable `3fe0b5b2412a79ee3deeaa90026c15da21f3807b` agents production CLI
was built under `target/s3b-b4b-production`, SHA-256
`ea867d1a6785ea7ada5f94c6af0177a28fd12210995e9d35327a37c981534765`.
Agent-ready smoke passed. Pinned QJS_CLI_BIN/QJS_AGENTS=1, binding/IC debug modes
unset, `find-qjsng-gaps.sh --exact --all --no-recommend --out-dir
 target/s3b-b4b-exact` completed: Rust **42,671 pass / 1 fail / 0 timeout /
0 not-run**, NG 42,602 pass / 70 fail; total 53,572, unchanged skips 10,900.
Summary SHA-256 `22e91a2ecd2d269bad0cafd67df1b5c4ed90a17b02a81194630da6c7023dc195`;
case-map SHA-256 `f27e633c23389659e2ddb18df687692c4b4349bd6f09582430c717353cf6b8d8`.
The complete regression scan is appended to burndown; it establishes no acceptance.

Sole gap: `test/language/statements/with/has-binding-idref-with-proxy-env.js`.
Frozen binary traces `Object:Object` for the empty Proxy environment; NG traces
`Object`. The legacy with miss path then called the typed resolver, selecting
HasBinding a second time. With loads/typeof/stores now use one typed selection
when a cursor exists. Added an exact Proxy trace runtime/NG fixture and the
upstream case to the curated allowlist. The after-fix complete with slice passes
**181/181**, NG 175 pass / 6 known nonpasses, zero gaps. This filtered scan does
not create a burndown record or replace final full cached/IC-off scans.

## B4c creation/activation ownership (in progress)

Realm, module/import/export routing, referrer, private environment and explicit
global selection have moved to shared CreationContext. ActivationContext owns
its call owner, scope entry/cursor, logical variable identity, execution mode
and strictness. Metadata-only view clones retain those owners; no lexical values
are added to either context. One empty import-metadata handle per root preserves
the existing Function API without allocating an empty map per query. Private,
module-host and import setters avoid copy-on-write when identity is unchanged.

Variable tokens are minted before VM scope exposure, in the existing shared
AtomDomain boundary so related realms cannot alias tokens. Scope instances use
the activation token instead of inventing one on first exposure. Parameter/body
boundary changes retain the token; strict eval selects its own holder identity.
Class regions save/restore typed mode/strictness, including suspension and abrupt
unwind. The temporary CallEnv Clone API still requires RefCell promotion through
&self; B5 removes that wrapper when Context borrows the activation mutably.
Function-owned creation selection and borrowed NativeEntry remain upcoming cuts.

The module-root flag belongs to ScopeRoots, independent of temporary execution
mode: a class-key region cannot disable canonical module-cell routes. Fresh
ordinary/eval variable entries clear that root role while retained views keep it.
With fallback now selects one typed reference; source names no longer carry
explicit marked-global control values in frame maps. The complete with slice
and its 7-case curated slice pass. Full runtime verification passes after
the final mode/root-role adjustment: **2,558 runtime tests**, **283 NG scripts**,
separate recovered module comparison, feature compilation and Clippy pass.
The 181-case full with slice and 7-case curated slice pass. Staged gate passes at `1e409f0e`, including all 5,171 curated Test262 cases.
Function-owned creation selection remains B4d; this is not borrowed-ABI
completion or final full-conformance/performance acceptance.

## B4d function-owned creation (in progress)

FunctionData now retains one shared CreationContext instead of separate realm,
module-host/import and private-environment ownership. Ordinary entries select
that exact creation owner, with a fresh variable token, and inherit no caller
name/value view. Source/host belongs to the function, not a caller fallback.
Native roots share a single weakly attached default creation record per Realm;
the attachment retains nothing and copy-on-write disassociates weak defaults.
The internal TDZ Function sentinel temporarily has no creation context; it is not
an executable user callable and its marker-facing compatibility is a B6 deletion.

Class private environments now exist before constructor functions retain creation
metadata. No post-publication Function private setter is needed. Nested heritage
selects the nearest initialized enclosing class environment while the new class's
own private environment is not active yet; keys then select the inner environment.
The constructor/prototype backreference no longer requires mutable creation state.

Deleted: dynamic_function_scope_snapshot, the AST/new_user/with-bytecode factory
family, per-call marked-global property copying, callee_this_realm_env, and forced
caller-host fallback on ordinary/constructor entries. Dynamic Function kinds use
one compiler and retain the callee Realm plus a live selected global object;
source text/WTF-16 spans reach nested bodies. Globals, getter/setter writes and
deletes use the selected object without publishing into the origin's cells.

The explicit synthetic-realm protocol retains typed prototype identities using
known constructor descriptors and the existing Object/Array identity slot, never
namespace values or getters. Boolean/BigInt/Symbol prototype intrinsics are now
set once alongside the existing Number/String intrinsics; primitive boxing no
longer follows rebound public constructor variables. This generalization is
required by removing marked namespace copies. No value/layout or object.rs
prototype-cycle/IC-admission ownership path is edited.

Verification so far: 2,562 runtime tests and 285 NG fixtures pass, including live
selected global getter/deletion behavior, prototype rebinding, nested heritage
private identities and dynamic nested source text. The weak-owner test, agents/perf-counters compilation, Clippy and separate module
comparison pass; final staged gate remains. The full runtime count is 2,563 with
that final identity test included. This is not the borrowed Context switch;
RuntimeServices, NativeEntry, generic saved references and final compatibility
retirement/stress/counters/complete exact cached+off/triage remain.

The staged gate caught env.rs exceeding its 2,000-line budget. RealmState,
realm construction schemas and the immutable eval-plan cache now live under
context/realm.rs, context/realm/construction.rs and context/eval_cache.rs.
Module import routing types live with CreationContext. The obsolete Object/Array
intrinsic snapshot-handoff reader/constants are deleted; typed creation records
supply those identities. The legacy env file now owns only the remaining ABI
compatibility to retire, rather than also owning Realm semantics.


## B2c1 selected-cell policy retirement

Deleted FunctionData/CompiledUserFunction immutable-name/environment-name/value
compatibility fields, the extra NewFunction policy flag, generator snapshot name
strings, class-name copies into FrameBindings and the associated admission guards.
FunctionNamePlan/ScopeLease and the selected BindingCell now supply self/class
write policy. Global lexical metadata cannot veto a mutable captured parameter
with the same name. Positional captures take precedence over module import-name
routing; an import's readonly policy applies only to that selected canonical cell.
Runtime and module regressions cover both homonyms. Initialization probes inspect
cell state without reading its value. All **2,565 runtime tests**, 286 NG fixtures,
Clippy and the staged gate including 117 selected Test262 cases pass. Borrowed Context and removal of named call inputs remain.


## B2c2 arguments demand boundaries

Compiler arguments demand now follows current operations and lexical arrow
children, using each operation's recorded scope. Ordinary children/methods,
fields and static blocks do not demand the parent's arguments object. Arrows
that only use parameters no longer synthesize a parent arguments binding or
mapped parameter cells. A demanded lexical arrow installs its owner's slot
before capture recompilation; transitive arrows and eval propagate demand.
Parameter/lexical homonyms stop it, with parameter-default eval retaining the
separate earlier scope. BindingPlan records inherited demand; own object demand
comes from its implicit Arguments declaration. No executable IR/source-shape
recognizer is introduced. All **2,568 runtime tests**, Clippy and the staged gate including 117 selected
Test262 cases pass. Lazy object recipes and
positional inputs for suspend/eval paths remain upcoming work.


## B2c3 positional inputs for every call kind

CallInputs now borrows the function and actual slice on general synchronous
entries. Generator/async entry owns positional actuals only until its initial
frame is constructed; no parameter/rest name/value map is built. Parameter,
pattern/rest and duplicate routes seed slots before cells. Mapped argument
indices retain those exact cells by compiled slot; the old argument-name
search/cell promotion and positional-admission exclusions are deleted. Rest
arrays are created once in frame construction with the selected realm prototype.
The existing direct-tier argument slice contract is unchanged. All **2,568 runtime
tests** pass, including updated admission tests for dynamic/suspended inputs and
mapped aliases across callbacks/storage reuse. Clippy and the staged gate with
117 selected Test262 cases also pass. Implicit arguments is temporarily
published as its canonical cell in the compatibility view; B3/B5 delete that
view and B2's lazy recipe will avoid materializing unread objects.
