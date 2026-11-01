# T077 S3b implementation record

Implementation base: `1e6e2a2b8ccdb590d735e5b6e969105cb1344c72`, accepted
S1+S2 integration (runtime `dc710e4b`, acceptance `fc3deef7`). Frozen brief:
`71294a20864662808216f874635dc8fb0473693e`. Owner branch:
`agent/t077-s3b-context/codex`. User authorizes local commits only, no push/merge.
The integrated S2 semantic API is the input; Context does not access NamedStore
internals. Prototype/layout follow-up paths remain outside this owner boundary.

Final comparison endpoint is accepted main **3cc12c6f** (S2 follow-up); the
cumulative rebuild endpoint remains `6663738f` per T077. Lead owns exact formal
binary/corpus/recipe pinning and formal/held-out/RSS acceptance. Round 4 ends at
frozen sha 8abd4c69 (runtime 0148b2eb), ready for lead acceptance as an enabling
stage: the [round-4 owner receipt](T077-s3b-final3-owner/README.md) owns the final
qualification and [public86](T077-s3b-final3-public86/README.md) its lane/case
numbers. No formal performance acceptance is claimed.

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
cell state without reading its value. All **2,565 runtime tests**, Clippy and the staged gate including
117 selected Test262 cases pass. The separate NG run in B2c4 corrects the two
new fixtures to the comparison runner's expression-only contract. Borrowed Context and removal of named call inputs remain.


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


## B2c4 canonical module declaration ownership

Modules compile their body and create typed declaration cells before linking can
publish a namespace/import route. Hoists and body frames attach to those same
cells. Imported settled values no longer enter the shared Realm; Function and
indirect eval therefore cannot see module import names. Namespace reexports
are resolved to namespace identity even through an indirect export. Removed:
import_bindings, module frame binding_snapshot/export apply-back, settled export
maps and global/undefined fallback, top-level-await frame reconstruction from
settled cells, and DynamicBindingsMut snapshot-on-borrow/apply-on-drop. Module
completion returns only its async promise; routes remain canonical throughout.
All **2,570 runtime tests** pass, including namespace alias identity, import
isolation and an undefined export shadowing Object. The staged gate (117 selected
Test262 cases), agents/perf-counters compilation and all **287 NG scripts** pass.
Separate module fixture `s3b-context-imports.mjs` (SHA-256
`91c9a512b922fb1dda4b9519a5c077863689c9469e265c05bd4121fa1c3a5b71`)
prints `7:undefined:undefined:undefined` then `9:9` in both engines under TZ=UTC.
The script runner requires expressions and flattens newlines; B2c1/c2 fixtures
are corrected to IIFEs here (their runtime tests had already passed). This moves source-module compilation before linking;
it preserves structured Syntax errors and retains the compiled body for reuse.


## B3a2 effect boundaries without captured-value copies

Removed conversion::to_primitive_method_env/sync_to_primitive_method_env and
BindingSnapshot. Ordinary coercion methods use their function-owned creation
and captured cells; no captured-name sorting, frame clone/scrub or binding
apply-back runs around them. Indirect eval's marked-global view no longer copies
any global own-property values; it retains the live selection. A regression
checks repeated getter reads and deletion without touching the origin. All
**2,571 runtime tests**, the staged gate (117 selected Test262 cases) and all
287 NG scripts pass. The no-longer-consumed recursive write-name cache and native
capture-key enumeration API are deleted with the coercion snapshot path.

The complete module-code slice at committed `d7a0db74` passes **595/595**, NG
589 pass / 6 known nonpasses, zero gaps/timeouts/not-run. Production agents binary
SHA-256 `673e6bc39c537495f70b57fd8c97eccfac72b555105ba52a40c15bd161cb852d`;
`--agent --raw -e "typeof __quickjsRustAgentStart"` returns `function`.
Summary SHA-256 `592bafe4c02d38d9c10e1e72ff376275a406cd62bd64f042ff6d9981f028faff`.
Case-map SHA-256 `55da6d79ae0db402cc12fdc9a5a75ae1e6168477b258f8439fd72dee12156a7a`.
This is a complete filtered slice, not a full scan/burndown or S3b acceptance.


## B3a3 live eval entry and projection/apply-back retirement

DirectEvalSite replaces the four direct-eval/strict/parameter/arguments marker
bindings; marker scrubbing, VmCallEnv and VmCallEnvOrigin are deleted. Own
arguments demand comes only from BindingPlan, so the duplicate parameter-shadow
admission flag/scanner is gone. Every direct/indirect eval now selects live
leases plus either an own strict holder, the caller's own variable holder or
the global record. Eval declaration cells attach by compiled root ordinal,
never a frame-name/value map. Global function declarations reserve their proper
property order before vars, validate descriptors before changes, and invalidate
accessor data routes without calling getters. No global lexical values escape
an eval's fresh declarative environment.

frame_call_env now retains typed scope/creation state and no values; initial
deopt overlays are no longer allocated. Removed: global eval writeback/shadow
repair, binding Snapshot/result lookup APIs, eval-deletable side names, all
apply_env/apply_call_env paths and native/callback apply-back. Completion carries
only result/throw and bound derived-this, not env/locals/cells. Existing compact
caller-environment tiers are still present until B5: the eval-only global-slot
mask distinguishes them from global eval entry. No tier is disabled.

Implementation checks caught a fast initializer skipping eval entry cells and
global eval slots using unused TDZ placeholders. Entry roots now decline that
initializer; object data routes attach canonical realm cells and accessor routes
retain semantic Get/Set. All **2,573 runtime tests**, staged gate (117 selected Test262 cases),
agents/perf-counters compilation and **288 NG scripts** pass. The new fixture
covers declaration effects, lexical shadowing and accessor-loop behavior. Its
two global-name results are sorted: NG and Node list var before function here;
our ordinary global object follows the step 17/18 declaration ordering. No
property-order equivalence is claimed. The complete eval slice remains next.

One prior assertion incorrectly rejected eval lexical shadowing of a global
function (`evalLexCollisionFn`). ES2025 EvalDeclarationInstantiation step 16
creates eval lexicals in the fresh environment without HasRestrictedGlobalProperty;
pinned NG and Node v22.23.3 both return `false:undefined`. The expectation is
corrected, with a new `let undefined` eval regression; this is a specification
correction, not a correctness waiver. Primary reference:
https://tc39.es/ecma262/2025/multipage/global-object.html#sec-evaldeclarationinstantiation


Pinned NG rejects `(0, eval)('let undefined = 11; undefined')` with SyntaxError;
Node v22.23.3 returns 11, as step 16 of the same ES2025 algorithm requires. The
compatible NG fixture shadows the newly declared function name instead; a
separate runtime test enforces restricted-property lexical shadowing. No NG
wrong result is encoded as an engine expectation.


## B3a4 deopt/bypass retirement

Deleted the flattened deopt map from FunctionData, CompiledUserFunction and
CallEnv; closure-scope Keep/Drop/Bypass, weak bypass registry/revocation, overlay
memos and projection builders, per-parameter marker-prefix splitting and its
capture shortcut are gone. The compiler's parameter/body variable boundary
selects the typed identity directly. The unused suspend/name-resolution
admission caches and execution snapshot/fork/visible-name APIs are deleted.
Canonical global/module tables live in binding/table.rs with a non-wrapping
mapping generation for the existing guarded scalar route cache; no execution
frame owns such a table. The virtual-function context check now uses live scope
and compiled dynamic-scope facts instead of the removed deopt-map proxy.

All **2,571 runtime tests** and agents/perf-counters compilation pass. Two tests
of the deleted overlay/enumeration API are retired; typed-view tests now verify
that derived-this/module identities survive without copied compatibility
bindings. Existing live-holder/lexical-shadow tests remain. The staged gate (117 selected
Test262 cases), Clippy and all **288 NG scripts** pass.

Complete exact eval slices at `eb846bf2`, production agents binary SHA-256
`eb660d0e7ef3ad46cef87d27719e976df79bf5fc1fcbf47da1c4819ee65b926d`:
language **347/347**, Annex B **469/469**, both engines pass every case; zero
skips, gaps, timeouts or not-run. Summary/case-map SHA-256, respectively:
language `1c00ef143a90bcdcb285b63273a76abda71c6df119f9f3c9ed61575b66130d31` /
`bc445992d178afc14068a63bf52255c0f7508dc3fe8a2330d233a8e6f6b47419`;
Annex B `5bf1e237e9391c74e6d39c7872e8b943a57b141f0c72e6087387f9240fd82b5c` /
`383b164a4662190d414a5fe23677327ce096045ffdd930603ee9a5265e03df5d`.
These filtered scans create no burndown record or final S3b acceptance.


## B2c5 lazy arguments value authority

Implicit arguments is initialized with a typed recipe; no Arguments object or
mapped getter/setter functions are created until a value read. Intrinsic,
prototype, callee and iterator identities are captured at entry without JS.
The recipe retains only actual argc values and supplied mapped parameter cells;
unmapped/missing/duplicate positions preserve their distinct rules. Unused
formal parameter cells are no longer allocated solely for arguments. Debug,
initialization probes and Assign do not read the recipe. The first value read
runs the pure factory once, outside the binding storage borrow.

Late-synthesized arguments references wire to the compiled slot without changing
instruction PCs. The canonical binding no longer publishes into FrameBindings.
Non-simple body-var copies use a compiler-recorded prologue route: distinct
parameter/body cells share one deferred object value and can be independently
reassigned. This also avoids the old eager LoadLocal/StoreLocal copy. Parameter
expression fallback slots stay separate from later body var declarations.
Design correction: the frozen Box-only recipe cannot represent a non-simple
parameter/body value copy without eager materialization or two factories.
A shared lazy-value Rc owns one factory while the two binding cells stay
independently mutable. This changes no executable IR type, Value representation,
dependency or unsafe code.

Before the final copy deduplication, **2,577 runtime tests** pass. Five focused
perf-counter/cell tests now prove: no object for direct/eval/body-copy overwrites;
one recipe/one materialization for copied values; no recipe for parameter-only
arrows; escaping identity and mapped cells survive suspension before first read.
Final default/feature/staged/NG gates follow. Owned suspend-start positional
inputs still remain until the borrowed entry cut; this is not S3b completion.


Full agents/perf-counters testing exposed one obsolete counter assertion from
the deleted marker-scrub protocol. It now asserts the Proxy/eval callback result
and zero scrub events. The lazy-copy independence case is another pinned NG
limitation: after `var arguments; arguments = 'body'` in a non-simple body,
the parameter-default arrow must retain its parameter arguments object. Node
returns true for identity with the body's pre-assignment value; NG returns false.
The existing runtime test enforces the approved distinct parameter/body cells;
the NG fixture compares identity before the reassignment and retains its other
mapped/argc/deletion/suspension checks.


After the wrapper interruption, branch/index/resume were checked at `a1575448`;
no unstaged or unrelated changes were present. A fresh staged gate passed
2,577 runtime tests plus 135 selected Test262 cases; repeated feature tests
passed 2,626 and the script comparison passed 289. Staged review then found that
late-wired implicit arguments slots must share the parameter-binding live-body
route for non-simple functions with eval. The route now uses declaration source
names (including internal storage slots), with a focused body-eval shadow/write
regression. The updated staged gate passed; the final common-path correction was then
reverified with 2,578 default tests, 135 selected Test262 cases, 2,627 full
agents/perf-counters tests and 289 NG scripts.


The body-eval shadow regression passes. Updated full feature tests pass
**2,627/2,627**; default gate passes **2,578/2,578** plus 135 selected Test262
cases. The maximum-concurrency script run hit a 10-second teardown timeout in
named-property-iterative-release.js while full feature tests were running.
A complete rerun with COMPARE_QJS_JOBS=4 passes all **289** scripts without
changing the timeout or expected results. Ordinary initialized cell reads keep
the original single-borrow path; only lazy reads enter materialization.
The final staged gate includes that common-path correction.


## B3b1 saved variable-environment bases

A saved extensible-holder reference now retains the holder/name, never a stale
cell or a parent lookup route. After RHS deletion, sloppy SetMutableBinding
recreates the own binding in that holder; strict missing assignment throws.
After own redeclaration it writes the current own cell. Static CellReference
identity remains unchanged, including the existing deleted-raw-cell test.
New lowest-layer tests cover replacement identity, unchanged parent/old cell,
strict missing/no creation and current-own reads. The permanent eval initializer
fixture reproduces delete/redeclare during RHS: Rust previously returned 2:1,
NG/Node return 9:1; the repair returns 9:1. A second spec test covers recreation
without RHS redeclaration. All **2,581 runtime tests**, the staged gate (65 selected Test262 cases),
agents/perf-counters compilation and **290 NG scripts** pass. The heavy teardown
fixture passes isolated and in a complete COMPARE_QJS_JOBS=1 rerun with the
unchanged 10-second timeout; a concurrent run timed out while another owner
was scanning Test262. General reference capture before RHS remains next.

Design correction: frozen wording that a selected reference keeps its old cell
is correct for a raw static cell reference. An environment-record reference
must keep its environment base. ES2025 Declarative SetMutableBinding step 1
explicitly handles a missing binding after RHS deletion. Holder is now a distinct
ResolvedReference variant, preserving this difference without snapshots or
reselecting a parent. Reference:
https://tc39.es/ecma262/2025/multipage/executable-code-and-execution-contexts.html#sec-declarative-environment-records-setmutablebinding-n-v-s

Complete exact arguments-object slice at `3a4173b0` passes **263/263** in both
engines, with zero skips/gaps/timeouts/not-run. Production agents binary SHA-256
`4e7625bf0ade7aad806db9b92e600c3ce9526795670398749f3bfe7e2622432b`;
summary `80d79e27758ae48f0102405e8a5e24c3a2bdc23d57d8f524486171ac27f410c6`;
case map `d3f84fe67ea9972b020213203a59419f57a2108c438174a5cfc2b4a06cbb21f9`.
This filtered scan creates no burndown or final S3b acceptance.


## B3b2 pre-RHS reference selection

Immutable BindingPlan reference sites retain the LHS scope before lowering its
RHS. Existing IR and PCs are unchanged. The general VM selects a crossing
reference before effects, consumes it at the recorded read/store, and releases
it after short-circuit/abrupt exit. Explicit with references share the lifetime
bank; read-only call references release after the callee/receiver reach the stack.
Generator snapshots retain the same reference and lifetime, including originally
missing names. Owned static slots, received cells without mutable scope chains
and undeletable global bindings retain indexed/tier execution; unstable sites
stay on the interpreter until their interval completes. No name/value snapshot.

GlobalReference retains the global environment and original resolvability.
Originally missing strict writes throw even if the RHS creates the property;
sloppy writes target that global object. Global eval excludes module imports,
and selected-global overrides are taken from the actual activation rather than
an empty service view. Declarative and object subrecords stay distinct.

Ten runtime tests cover closer eval declarations, const policies, holder deletion,
strict missing writes, yield, ToNumeric callbacks, destructuring defaults,
class-name scope entry, repeated logical assignments and catch/reentry. A metadata
test proves the recorded LHS scope precedes the RHS class scope. The compatible
NG fixture covers short-circuit/reentry/catch and with-call receivers. Primary
semantics: ES2025 13.15.2 evaluates lRef before rRef and later PutValue uses lRef;
9.1.1.1.5 handles a missing binding in the already selected declarative record.
NG and Node currently re-resolve several crossing-reference probes; those spec
results remain independent unit tests, not fixtures encoding their wrong answer.
Verified over `3999f127`: all **2,592 default runtime tests**, agents/perf-counters
compile, the staged gate including **65 Test262 cases**, and all **291 NG scripts**
with COMPARE_QJS_JOBS=1 pass. Final S3b gates remain open.


## B3a5 compatibility value-container deletion

At `ae923849` the general reference unit is committed. This subsequent audit
removes FrameBindings/FrameBindingValue completely, not merely their eval use.
The remaining indirect global-eval hoist inputs are no longer copied into a
name/value vector. Hoists attach the existing canonical global data cells or
use semantic object routes for accessors. Retired: marked_dynamic_realm_snapshot,
restore_marked_dynamic_realm, refresh_realm_backed_locals_from_realm, frame
publish/remove/replace APIs, global-value equality synchronization, and the
obsolete vector-promotion test. Sloppy fallback loads select the live global
route, so callback writes/accessor replacement/deletion need no post-call pass.
The permanent global-effects fixture verifies all three effects with NG.

Actual CallEnv struct construction (excluding Clone), Clone::clone and indexed
tier compatibility proof attempts now have separate feature-gated counters.
They count real events before B5 deletion, not allocations or delegated factory
calls, and remain visible as zero-event audit fields after the migration.
No production timing claim follows from these diagnostic counts. All 2,641
agents/perf-counters runtime tests and all 292 serial NG scripts pass. The staged
gate over `ae923849` passes 2,592 default tests and 117 Test262 cases. Borrowed Context, checked-publisher
retirement, lazy promotion and the final matrix remain outstanding.


## B4e1 pure function descriptor services

Over `bc6d6c16`, function descriptor walks no longer construct a creation_env
or a fresh owned function-entry service object. The selected [[Prototype]] is
already installed; walking it is pure. Lazy default prototype creation selects
CreationContext's typed intrinsic override or the stable realm Object.prototype
directly. The obsolete with-env prototype wrapper is removed from these paths.
All 2,592 default runtime tests pass, and an agents/perf-counters regression
proves inherited, missing and lazy prototype descriptor reads construct/clone
zero CallEnv instances. All 292 serial NG scripts and the staged gate (2,592
default tests plus 52 Test262 cases) pass. This is a
service-boundary deletion, not a new property cache or production timing claim.


## B4e2 stable internal error materialization

Over `37982ae9`, realm error constructors/prototypes have typed set-once slots.
VM catches, internal rejection conversion and RegExp receiver errors construct
from the selected intrinsic prototype, without public-constructor lookup,
coercion, native entry or an empty service environment. SuppressedError's
internal constructor selection is stable too. User-requested constructor calls
retain their normal coercion/cause/construct behavior. RegExp no longer captures
its entire global object for error construction, and NativeContext clone_map is
removed now that no execution consumer needs it.

Explicit synthetic marked globals preserve their protocol: function-level
selection captures typed intrinsic identities together with GlobalSelection;
private-brand metadata selects its defining error prototype before later marker
changes. Ordinary public constructor rebinding cannot redefine the realm slots.
The private error object's own message/ErrorData now use the same pure builder.

All 2,593 default tests pass. The new NG fixture replaces TypeError, RangeError,
ReferenceError, SyntaxError, URIError and SuppressedError, then checks engine
errors keep their intrinsic prototypes and never invoke the replacements.
A feature-counter test checks internal rejection conversion retains prototype,
message and ErrorData with zero CallEnv construction/clone events. All **2,644
agents/perf-counters tests**, **293 serial NG scripts**, and the staged gate
(**2,593 default tests + 117 Test262 cases**) pass. Final S3b remains open.


## B4e3 typed native closure payloads

Over `6bda819b`, NativeCaptures replaces NativeContext's string/value HashMap.
Promise capability/resolution/finally/element reactions, async function/generator
continuations, async-from-sync wrappers, Array.fromAsync reactions and RegExp
getters retain concrete product variants. Indices are usize, done is bool,
arrays/state/guards are typed handles; JS callback/value fields remain Value.
Shared guards and retained object identities are unchanged. No payload retains
an execution facade. Captures are installed only while Function identity is
unique, then retained immutably through that identity.

Plain natives have None and allocate no capture payload or shared empty context.
The obsolete per-domain weak empty NativeContext attachment, map insertion/COW
and map access APIs are deleted; property domain comes from creation ownership.
The payload allocation counter counts actual Rc::new sites, independently from
function creation or old CallEnv construction. Existing lifetime/header tests
now enforce absent empty payloads and unchanged domain lifetime. A counter test
separately verifies zero empty allocation, one real payload and its retained value.

All **2,646 agents/perf-counters runtime tests** and **294 serial NG scripts**
pass. The new fixture checks duplicate-call guards and indexed values through
Promise.all/allSettled/any with a generic synchronous constructor. Source audit
finds zero NativeContext/native_context/insert_native_context references under
runtime src. The staged gate passes **2,594 default tests + 52 Test262 cases**;
the atomic Context ABI cut remains.


## B5 atomic borrowed Context ABI

Implementation over `eda5e1cb`: NativeEntry and all runtime consumer families use
Context borrowing RuntimeServices and an actual ActivationContext. NativeSite
borrows CallInfo, distinct target and direct-eval scope/private sources. Native
entry allocates/clones no activation, creation record or realm; the focused
128-call counter test verifies zero old construction/clone/equality events and
zero variable tokens. Native realm/target/caller ownership has separate identity
and Rc-owner assertions. Direct eval no longer mutates/restores caller site state.

Vm borrows services; compact/wide stacks save each callee activation, creation,
strictness and variable identity. All caller-environment equality proof/entry
APIs are removed. Numeric-plan internal frames now have fresh logical identities;
probe counts include rejected speculation, while successful retained callee counts
alone join ordinary attempts. The recursive 31/63-call receipts still count the
actual call tree. Numeric/this closed leaves mint their logical token too.
RuntimeGroup uses the existing AtomDomain boundary and weak global/realm metadata,
with amortized cleanup; global notifications find communicating foreign realms.
Host/referrer and agent services keep their canonical owners, without new queues.

Removed: CallEnv definition/Clone/Debug/re-exports, function/env.rs and its
owned-view tests, vm_call_env factories (vm_context now only borrows), inherited
function with-stack copies, construct-target setter/restoration and OwnedCallInputs.
Generator prologues/initial async execution consume borrowed inputs synchronously;
no extra async snapshot was added. Snapshot/by-value continuation reconstruction
remains S3c/S3e-owned and retains typed owner/lease identity, never Context.
EvalOutcome retains services plus creation for host-controlled job drains.

Twelve tests for retired owned env/view APIs are replaced by eight typed ownership
checks plus a native-boundary counter proof; required JS regression tests remain.
Two eval-cache tests move to direct_eval/cache.rs to preserve the global test-file
budget. No dependencies, unsafe, new executor, new depth limit or benchmark-shaped
admission was added. Protected S2 owner paths are unchanged.

Verified over `eda5e1cb`: **2,590 default runtime tests**, **2,643 agents/perf-counters
tests**, **294 NG scripts** with COMPARE_QJS_JOBS=1, strict workspace feature
Clippy and staged gate with **117 selected Test262 cases** all pass. No runtime
consumer retains CallEnv, FrameBindings, an owned native environment factory or
a caller-env equality entry. Only retired event descriptions name CallEnv.
Separate TZ=UTC module comparisons all exit 0 with identical stdout. Main output
hashes: deferred-key-arguments `8b839ca4974d1086d3d9105322e1372606c8d146d0053541eb6b65527c295ac8`;
s3b-context-imports `bb0199327236bb286b72263d3b5678affe852f0cb0fe2f392782e88533301da5`.
Source hashes/commands remain as recorded above; dependency module stdout is empty.
Raw cell publisher retirement, default lazy promotion, stress/deletion/counter
receipts and the final full/exact/public86 matrix remain before S3b exit.


## B2c6 checked compound writes and canonical module publishers

Over `027df1e5`, module writes use their canonical cell without a second
name-routed export publication. Wide upvalue stores use checked Assign.
String literal compound assignment now checks immutable policy after required
ToPrimitive; global lexical string appends cannot bypass const through mutable
cell access. The unchecked mutable cell callback API is removed. Sloppy named
self assignment still returns the computed value without changing the binding.
The feature suite passes **2,645 tests**, and all **295 serial NG scripts**
pass. The fresh staged gate passes **2,592 default tests and 65 Test262 cases**.
Final S3b remains open.


## B3b3 lazy slot promotion and scope-instance authority

Over `fe437a9a`, private captured/eval-visible declarations remain in indexed
slots until the first actual capture, saved reference or scope exposure. The
promotion moves the value/TDZ state, clears the slot and reuses that cell for
later exposure. Canonical incoming/module/global/mapped-arguments cells retain
their identities; entry and every checked local store clear their slot copies.
String literal append operates directly on a checked shared cell without a raw
slot refresh/writeback. Iteration cell replacement copies typed state directly.

Scope re-entry drops only private frame routes, preserving escaped cells even
when the earlier instance never initialized. Declaration Init in the same scope
keeps an earlier closure's cell. A driver RefreshBindings boundary reselects the
existing instruction stream after promotion, without replaying instructions,
reference HasProperty or hooks. Existing SRA analysis excludes captured/dynamic
slots; unrelated admitted virtual values and tiers remain enabled. Snapshot
restore reconstructs the active scope position and canonical routes.

The new lowest-layer test checks absent entry cells, moved value/TDZ and ptr_eq
capture with no retained slot. A permanent runtime/NG case separates an escaped
uninitialized iteration from the later initialized instance. All **2,647 feature
runtime tests**, **296 serial NG scripts**, and the fresh staged gate
(**2,594 default tests + 65 Test262 cases**) pass. Final S3b remains open.


## B2c7 checked reads/publishers and object-route invalidation

Over `3ac36ec3`, BindingCell has no raw set/get or mutable-value API. Declarative
reads return a structured TDZ error; cache probes return Option without creating
a Function sentinel. Init/Assign remain distinct. Numeric leaf writes validate
all destinations before publishing speculative results and use checked Assign;
sloppy immutable self routes decline to the semantic interpreter when needed.
Mapped argument getters and module namespace reads use the same checked cells.

ObjectStorage identifies global/host data caches separately from declaratives.
Only those cells accept object publication/invalidation; global lexical values
cannot be overwritten by a same-named object's notifications. Invalidated object
routes have their own state and re-resolve the live global binding/property.
Derived this starts directly in typed uninitialized state, with no marker seed.
The remaining wide-register-only sentinel belongs to existing tier storage; no
binding/context/VM local or global cache manufactures it.

Global literal string append now checks the property's writable descriptor and
uses semantic store for rejected routes, including sloppy ignore/strict throw.
Global lexical TDZ precedes a same-named property during function/typeof reads.
New cell policy tests and runtime/NG cases cover these paths. Module namespace
changes in value/object.rs are limited to checked export reads; prototype-cycle
checks, value/layout and IC constant admission are untouched.

Fresh feature runtime suite passes **2,650 tests**; all **297 serial NG scripts**
and the staged gate (**2,597 default tests + 117 Test262 cases**) pass. Final S3b
acceptance, stress controls and full/exact/counter/public86 gates remain open.


## B6a binding stress controls and retained event accounting

Implementation over `a5f15790` captures QJS_VERIFY_BINDINGS,
QJS_FORCE_EVAL_EXPOSURE and QJS_POISON_RELEASED_SCOPES once per RuntimeServices.
Production ignores forcing; constant-false release queries remove per-op verifier
work. Verification checks slot/cell exclusivity, metadata policies and received
cell order without reading values or replaying JS hooks. Snapshot restoration and
promotion run the same checks. Force moves every active materialized declarative,
including TDZ, into its cell. Scope/slot recyclers poison only private released
storage; reuse checks/clears poison. Escaped cells/leases are never poisoned.
The new identity test alternates lazy/forced entry and churns recycled vectors.

Design correction: forcing preserves existing compact/wide/closed numeric entry
whose admission proves own bindings cannot escape or be dynamically resolved;
it forces actual VM slot storage. Disabling those tiers just to force synthetic
cells overflowed the existing deep-recursion test and would violate the stage's
no-tier-disable boundary. Admission/counter tests explicitly choose slot mode;
semantic runtime tests and permanent fixtures inherit forcing. Wide-to-VM entry
now installs values into forced cells, then handback reads those same cells.
No production admission restriction or new depth guard is introduced.

Forcing exposed a parameter clear bug: own parameters marked from_env retained
a promoted initialized cell after their input snapshot was cleared. Clear now
distinguishes positional received upvalues from own parameters, preserving
parameter TDZ before defaults. Existing self/later default tests protect it.

New independent counters count actual CellStorage/ScopeLease/lexical-owner
allocations, function scope-retention events, and existing local/operand/cold
pool requests separately from new banks/boxes. Operand initial-bank counts do
not claim Vec growth or allocator-wide totals. Retired CallEnv counters retain
zero sites; none are relabeled as typed work.

All **2,598 default tests** pass separately in verify, force and poison modes.
All **2,651 agents/perf-counters tests**, strict feature workspace Clippy and
all **2,598 tests with IC verification enabled** pass. Combined binding modes
plus IC-off pass **2,561 semantic/identity tests**; 37 IC-enabled mechanism tests
are exercised in the complete IC-verification-on suite instead. The unfiltered
IC-off experiment failed 26 such mechanism assertions, with no semantic failures;
no test/fixture or Test262 exclusion is added. All **297 serial NG scripts** pass
in each verify+IC-verify, force, poison and combined+IC-off configuration.
The fresh staged gate passes **2,598 default tests + 117 Test262 cases**.
Final full/exact/counter/depth/public86 and lead formal acceptance remain open.


## B4e4 preserve intrinsic error origin across context release

Over `281b005d`, actual two-realm tests reproduce foreign native RangeError and
foreign JavaScript TypeError acquiring the caller's intrinsic prototype after
return. Native entries, specialized native calls, VM error propagation and both
retained tier unwind paths now materialize internal errors while the originating
callee Context is live. Explicit thrown values keep their identity; the pure
intrinsic builder invokes no public constructor or JS hook. The private wide
TDZ sentinel has no callable realm and uses its active activation's error origin.
The duplicate VM message-to-constructor path is removed.

Three permanent ownership tests cover native toFixed, a dynamically created
foreign function and a foreign decodeURI invoked through a caller-defined body.
The first two fail before the fix; the specialized-call test protects the native
shortcut as well. All **2,654 feature tests**, **297 serial NG scripts** and the
fresh staged gate (**2,601 default tests + 65 Test262 cases**) pass. Final S3b
acceptance remains open.


## B6b logical-entry denominator and explicit retired events

Over `eca56b86`, diagnostic ordinary_call_attempts counts one entry when the
owned-argument host dispatcher forwards to a direct leaf. Previously both the
host dispatcher and leaf entry incremented it. The admission check used solely
for this attribution is feature-gated and absent in production code. A separate
ordinary_call_forwardings event retains the observable routing count. A focused
counter test proves four entries for an outer function plus three map callbacks,
and one entry/one forwarding for an owned host call.

Explicit zero-site counters retain the named snapshot/fork/apply-back boundaries
and S3a native argument-pop/family-probe/decline events. They identify removed
work; typed cell/lease/owner and recycler events remain separate. The retired
caller-environment substitution counter description is corrected. Clean accepted
S2 and final candidate ten-program maps are collected separately, twice each;
no timing is inferred from counter builds. Fresh **2,655 feature tests** and the
staged gate (**2,601 default tests + 52 Test262 cases**) pass.


## First freeze rejected at the depth/counter audit

Runtime `0b659b89` passes the full check, but does not meet the depth matrix.
The immutable [round-one receipt](T077-s3b-round1/README.md) owns its complete
base/candidate maps, depth traces, prologue evidence and canceled public86 job.
It establishes zero old environment events, not final payoff or acceptance.
Current uncommitted repairs keep interpreter fallback and inline-entry move
temporaries outside retained driver frames, borrow the actual prepared callee,
and avoid fixed-name lease propagation without compiled free-name demand.
No resource limit, caller-context substitution or benchmark recognizer is added.
Fresh gates and a new source freeze are required.


## Entry and static-scope repairs over the rejected first freeze

The admitted entry borrows its actual callee through ownership slots. Only a
rejected tier moves that record into the outlined VM fallback. Inline-entry
preparation/move temporaries live in separate helpers, not across recursive
native callbacks. Scope transition work is outlined behind its unchanged-scope
guard. Existing executors, admission ordering, frame bounds and native stack
policy remain; there is no caller-context equality route or activation pool.

Fixed declarative mappings retain a lease only for compiled unresolved-name
demand. Extensible holders/with objects always retain live crossing lookup;
a function with potential own direct eval exposes its holder before later
additions. Pure parent scopes no longer allocate unused extensible holders.
Named functions retain self bindings for actual static/dynamic demand, including
all descendants, rather than merely because they create an unrelated closure.
Class constructors, methods, fields, static blocks and private members use the
same demand rule. The existing sorted/transitive global-name index is reused.

Deletion was absent from that index and raw-slot presence checks missed canonical
cells/TDZ. In the rejected binary, named-self/captured/TDZ deletion all returned
true versus NG false. Delete now participates in compiled demand, and the VM
checks active descriptors/received declarative identity without GetValue.
Unresolvable and object-record deletion retain their semantic paths.

Successful wide numeric roots also count their actual ordinary entry; retained
children remain separate. New identity tests cover unrelated fixed/self scopes,
descendant self references with inactive shadows, class consumers and deletion;
two permanent NG scripts protect the semantic boundaries. Preliminary checks
pass **2,661 feature tests**, **2,606 default tests**, **299 NG scripts**, strict
feature Clippy and the staged gate (**117 curated Test262 cases**). Forced and
poisoned binding modes each pass all 2,606 default tests. All sixteen accepted
S2 debug/release depth maxima are preserved; repaired maxima are at least the
base in every case. The attempted curated delete-only filter selects no cases;
its upstream slice will be exercised through the exact harness.

[Lead feedback round one](T077-s3b-lead-feedback-1.md) requires committing this
repair, merging main `3cc12c6f`, and then regenerating all final evidence against
that new baseline. The pre-merge depth/counter standing is not merged-source
standing. Origin pushes and merges into main remain forbidden.


## Lead feedback round one: merge accepted S2 follow-up

The fresh interrupted-step audit/gate passes before repair commit `539d6b8f`
(2,606 default runtime tests + 117 curated Test262 cases). Main `3cc12c6f` is
merged next as required by the lead. Conflicts retain both new function-cycle
tests and S3b ownership tests, and append main's exact burndown rows byte-for-byte
without editing or dropping prior records. T077's resume combines accepted
follow-up standing with unaccepted S3b standing. Iterative allocation-free object
and function cycle checks from main are preserved. Fresh merged-source checks
are required; the old `1e6e2a2b` measurements remain historical only.


The merged working tree passes the fresh fast gate: **2,636 default runtime
tests + 117 curated Test262 cases**. Both function-cycle tests remain enabled;
value/layout and named_property_cache submodules match main exactly. In
value/object.rs, the only S3b delta against main is checked module namespace
export access, and its cycle walker is unchanged. Final comparison baseline is
now **main `3cc12c6f`**. The x86 order must be regenerated before triage.


## Merged-source x86 order

The required default external/sentinel/broad order regeneration completes on the
iMac at source `4f6c0444`, job `20261031T232541Z-t077-s3b-merged-order`: 76 cases,
485 symbols, exit 0. [Order evidence](T077-s3b-merged-order/README.md) owns source,
binary/producer/order hashes and the exact command. No development/held-out
profiling or layout offset search is added. The final x86 timing binary must be
rebuilt with this order. Baseline for every new comparison is `3cc12c6f`.


## Owner checks at the merged/order-regenerated freeze

Runtime **`73c93d8f`**, baseline **`3cc12c6f`**. The [owner check receipt](T077-s3b-owner-73c/README.md) records complete two-run development maps, metadata,
all depth traces/landmarks, module matrix and deletion audit. The full gate passes
at this source (5,171 curated cases), 2,636 default/2,691 feature runtime tests,
299 NG scripts in each default/off/verify/force/poison/combined mode and all module
comparisons pass. Every baseline debug/release success maximum and landmark is
preserved. Production default/agents/perf-counters builds pass on both targets.

The complete cached exact scan rejects this freeze: 42,669 passes / three failures,
zero timeout/not-run; the complete burndown row is appended. IC-off discovery of
this rejected source was stopped, with no partial burndown. [Checkpoint](T077-s3b-exact-73c-rejected/README.md) owns the failures and diagnosis. Initial merged public86 job
`20261031T233745Z-t077-s3b-merged-public86` exits before measurement: policy expects
`iMac.local`, actual is `iMac.lan`. Identity job
`20261031T234927Z-t077-s3b-host-identity` confirms all other host/recipe fields
identical. The proposed alias-only tooling change is pending explicit approval;
no policy bands or identity guard have been bypassed. Final owner handoff remains
pending repair/refreeze, complete scans and valid public86 improvement.

## Exact-checkpoint repairs

Free-name sloppy writes now select canonical global lexical cells before the
object-cache route, including bindings created by host script entry after caller
compilation and a nonextensible global object. The checked publisher remains
object-only. A focused runtime test covers mutable let/class, const rejection and
absence of object properties. The synthetic host shim now uses a callable
nonconstructor for its Function prototype; the previous ordinary function's own
prototype property could leak into bound wrappers through typed creation data.
This changes no exclusion or expected failure. All three previously failing
upstream cases are added to the curated allowlist and pass after repair.

Repair gate: **2,637 default runtime tests and 5,174 curated Test262 cases**
pass, along with formatting, strict workspace Clippy and context checks. Full
exact affected slices pass **100/100 Function.bind** and **42/42 global-code**,
with zero gaps/timeouts/not-run. These filtered slices create no burndown rows.

## Repaired runtime and order freeze

Runtime `1e91be92`; final order candidate **`ca5ab643`**, baseline main `3cc12c6f`.
The [new owner receipt](T077-s3b-owner-ca5/README.md) records fresh full gate,
2,637 default / 2,692 feature tests, all 299 NG scripts and three module fixtures
in six modes, individual and combined stress, depth maxima/explicit parity,
ten deterministic two-run counter maps, deletion audit and all production builds.
All counter maps equal the prior candidate; retired events remain zero. The
[new order job](T077-s3b-repaired-order/README.md) profiles the unchanged default
76 public cases, producing 496 symbols, with no held-out input or offset search.

Complete exact cached/off is running on the hash-pinned repaired agents ARM
binary. The x86 order is not an ARM build input, and its post-order rebuild is
byte-identical; runtime/harness/configuration are unchanged between both source
revisions. Public86 still requires the pending explicit hostname qualification
decision. Neither partial exact scans nor missing timing establish acceptance.

Repaired cached exact is complete: **42,672/42,672 configured passes**, zero
failure/timeout/not-run/gap, unchanged 10,900 configuration skips and no lost
baseline passes. [Cached receipt](T077-s3b-exact-ca5-cached/README.md) owns raw
summary, commands and hashes. Its burndown is appended at runtime `1e91be92`;
the prior rejected row remains. IC-off is running on the same frozen binary.

Final IC-off exact is complete and identical to cached: **42,672/42,672 passes**,
zero failure/timeout/not-run/gap, unchanged exclusions and no lost passes. Its
complete burndown is appended at candidate `ca5ab643`; both scan processes finish
normally and generated submodule case files are removed. The final full gate
rerun passes. [Owner handoff](T077-s3b-owner-handoff.md) now owns the current
completion/blocker state. Fresh candidate-host audit at 09:24 CST still finds
only the hostname mismatch. Public86 has no valid measurements, its alias-only
qualification proposal remains unapplied pending explicit decision, and no
performance/stage acceptance is claimed.

## Round-2 repairs and public correctness gate

Main's hostname normalization is integrated by lead merge `55f5d1d6`; the old
alias proposal is historical and never applied. R1's inline/VM receiver split
is repaired in `28228ff1`, deleting receiver side channels and using one call
owner on every route. The public sweep discovers a pre-existing high-capture
shift panic, repaired in `78e8ebc3`. R2's discarded numeric-call bug is repaired
independently in `a6de336d`, with an explicit effect model and pass audit; its
numeric files/tests alone also pass on a main source archive.

[Round-2 handoff](T077-s3b-round2-final/README.md) owns final evidence: 2,670
runtime/65 curated fast gate, 302 NG scripts and four modules in three modes,
focused owner/effect/mask tests, and every public workload (86 cases, 258
case-modes, 351/351 invocations). First failed sweep remains recorded. The final
verifier keeps IC checks, debug assertions and overflow checks enabled; no
deadline/case/output is waived. No held-out program or timing is run. Full/exact
requalification is deliberately deferred until remaining lead review findings,
then one requalification; earlier receipts are source-bound history.


## Round-3 retained-global repair

The [seeded triage record](T077-s3b-round3-fuzz/README.md) records the final
candidate-only stale-global bug and its incomplete-first-patch discriminator.
Realm removal now invalidates retained ObjectStorage cells on every deletion or
accessor-replacement route; static captures fall through to current global
selection. Default/off/verify match NG and main `3cc12c6f` on both permanent
script/module regressions. Focused runtime and staged gate pass 2,695 runtime
and 65 selected Test262 cases. Corrected receiver generation, fresh seeded
triage and final qualification remain; historical receipts are not current.


## Final corrected differential run

Runtime `e1405670`; corrected receiver counters in the accompanying tool commit.
[Final differential receipt](T077-s3b-final-differential/README.md): 6,000 programs,
36,000 invocations, stable hashes; zero mode disagreement/panic/crash/timeout.
Receiver 2,000/2,000 matches all six engines/modes; all 44 invalid-counter seeds
now run. Raw overlapping totals: MATCH 3,199, FIXED 2,570, PRE 535, CHANGED 395,
REGR 31. Original runner exit 1 is retained. All 742 disagreement programs are
triaged against an independent Node script oracle and binding/try semantics;
only HasProperty trace differences and an inherited CLI error-label/index entry
remain unmatched. All Get/Set sequences agree with Node. No unexplained new
semantic difference remains; final qualification follows on this runtime.


The first final NG fixture pass found a fixture-transport error, before a
semantic comparison: the new property-equality receiver fixture included line
comments, while compare-qjs flattens source newlines. Removing the stale
before/after inline comments preserves executable JavaScript and allows its
closing braces to reach the parser. Runtime and generated engine binaries are
unchanged. The failed default pass is retained; all fixture modes rerun below.


## Final owner finish: correctness qualified, payoff missed

Runtime **e1405670**, final order/candidate **8413f925**, baseline **3cc12c6f**.
[Final owner receipt](T077-s3b-final-owner/README.md) records the complete standing:
full gate **2,695 runtime / 5,174 curated**, full agents **2,706** and counters
**2,739**, five debug stress suites **2,695** each, combined IC-off **2,653** with
42 IC-enabled mechanism controls covered separately; **318 NG scripts** in seven
modes and six module entries plus two dependency smokes per mode. Both production
targets/default/agents/perf-counters build and smoke; no dependency or unsafe added.
The fixture's initial newline-flattening parse failure is retained and repaired
without changing the runtime. Public correctness is **351/351** invocations across
86 cases / 258 case-modes. All **31** depth maximum/landmark probes preserve base
success, and all ten development sources/stdout/full counter maps reproduce twice
per side. Retired environment events remain zero; new allocation/work events stay
separate and do not imply a speedup.

Both complete exact cached and IC-off scans exit 0: **42,672/42,672 Rust passes**,
zero fail/timeout/not-run/gap, unchanged **10,900** configuration skips; NG
**42,602 passes / 70 failures**. Their 53,572-row case maps are byte-identical to
the accepted-S2 map. Complete burndowns are appended at runtime **e1405670** and
tool/source **7ce2ab30**, with unchanged ARM runtime/harness/binary inputs; the
final x86 order is irrelevant to ARM. No partial scan or historical row is replaced.

[Final differential](T077-s3b-final-differential/README.md): **6,000 programs /
36,000 invocations**, 2,000 per profile, stable hashes and zero mode disagreement,
panic, crash or timeout. Raw overlapping tags remain MATCH **3,199**, FIXED
**2,570**, PRE **535**, CHANGED **395**, REGR **31**, exit 1 retained. All 742
PRE/CHANGED/REGR programs are triaged; reference HasProperty/finally/eval/catch
and assignment-order differences are explained with spec rules and independent
Node observations. Only inherited CLI error-prefix spelling/index remains outside
those semantic matches. Receiver **2,000/2,000** matches; all 44 invalid counter
programs now run. No unexplained candidate-only semantic disagreement remains.

[Public86](T077-s3b-final-public86/README.md), job
**20261101T043615Z-t077-s3b-final-public86**, is complete and healthy: **430/430**
valid attempts, zero invalid/heavy samples, budget met, original verdict **likely
loss**. Every lane regresses in both wall and instructions; the linked record owns
all lane ratios (portfolio **1.161922 / 1.184748**). **S3b misses its payoff
requirement and is not accepted.** No tuning or timing retry follows in this finish.
Lead owns formal/held-out/RSS, re-review and disposition before dependent stages
advance. Existing inherited/next-stage owners remain. Owner commits this evidence
and stops; no origin push or main merge.

## Round-4 milestone 1: causes 1 and 2

Commits **aea492db** (cause 1) and **f609c0ae** (cause 2) over `b33a3819`;
baseline `3cc12c6f`. Plan: [recovery](../proposals/T077-s3b-performance-recovery.md),
[attribution](T077-s3b-r4/attribution.md). Codex's unmeasured WIP was set aside.

**Cause 1.** A bound global reference behaves as late resolution: PutValue on a
global environment reference consults the declarative record, then the object
record (ES2025 9.1.1.4.5), and so does a fresh lookup. Only a pure store whose name
is unresolvable at its start can diverge (sloppy: a lexical or property created by
the RHS; strict: the RHS creates the name); compound/update/append sites read at
their start and throw first. `reference_admission.rs` proves per activation which
starts need nothing (lexical, import, own global property), cached by realm,
imports, binding generation and global-object layout. Unproven starts are selected
before the RHS on every tier: the VM as before, typed loops via the A2 sink fix,
the register tier through `ReferenceCheck` checkpoints whose selections survive
exits into the interpreter continuation. Silent rejections now count and trace
(`indexed_binding_admission_rejections`, `reference_blocked_loop_edges`,
`scope_blocked_loop_edges`, CFDECLINE/TLBLOCK). `tests/global_references.rs` covers
an RHS-created lexical, delete/recreate, strict missing names, eval-declared globals
and delete/recreate before the callback, at top level, cold and on the register tier.

**Cause 2.** Each call still owns its activation; the identity is minted on first
exposure; owner inputs come from one borrow and only methods allocate class state.
Inline callees run in `ActivationStack` slots (no moves; Context rebound only on a
depth change; the root activation is never displaced). Returned unexposed
activations are rebound to the next identical entry (same creation/kind/strictness,
class state rewritten), across callbacks and runs through a bounded services pool.
`tests/call_activations.rs` covers alternation of methods, strict/sloppy,
constructor/plain callees, owner-exposing callbacks and inline errors.

Reproducers, local arm64 instructions vs base (median of 3, `/usr/bin/time -l`):

| Reproducer | before (`b33a3819`) | aea492db | f609c0ae |
| --- | ---: | ---: | ---: |
| bitand-global / partial-global | 39.40 / 25.15 | 1.018 / 1.017 | 1.019 / 1.016 |
| validate-like | 2.015 | 1.076 | 1.069 |
| upw0 / upw1 / upw4 | 1.539 / 1.512 / 1.487 | 1.551 / 1.523 / 1.497 | 1.164 / 1.168 / 1.177 |
| call-implicit / call-var global write | 3.179 / 1.570 | 1.498 / 1.317 | 1.500 / 1.319 |

The two global-write call reproducers' remainder is the global-code VM loop
(per-op reference hooks, by-name routing: cause 5). Local 55-case instruction
geomeans, development/external: before 1.0806/1.2553, aea492db 1.0791/1.0792,
f609c0ae 1.0354/1.0526.

Public86 quick triage `20261101T090800Z-t077-s3b-r4-m1` (f609c0ae vs 3cc12c6f),
healthy, budget met, verdict likely loss; lane geomeans of case ratios:

| Lane | Wall | Instructions | Final (`20261101T043615Z`) |
| --- | ---: | ---: | ---: |
| development | 1.041 | 1.034 | 1.087 / 1.108 |
| external | 1.051 | 1.054 | 1.253 / 1.276 |
| broad | 1.061 | 1.079 | 1.079 / 1.096 |
| sentinel | 1.006 | 1.052 | 1.005 / 1.052 |

Worst: date-format-xparb 1.686/1.604 and string-tagcloud 1.540/1.466 (cause 3);
lodash 1.197/1.093, raytrace-public-class-fields 1.159/1.148, binary-trees
1.134/1.154 and broad call cases about 1.15 (remaining per-call activation work).

Checks at f609c0ae: fast gate 2,708 runtime + 117 curated Test262; perf-counters
2,756; compare-qjs 318/318; public sweep 351/351; differential seeds 0–299 × three
profiles, 900 programs with sources and tags identical to the final qualification
for every seed and no mode disagreement; reference-sensitive Test262 slices (17
prefixes) 3,779/3,779 cached and IC-off; depth maxima forEach 951, map 942, reduce
1,000, call 3,567, apply 2,335, reflect_apply 2,492, yield_star 831, plain 4,096
(all ≥ base). No timing ran locally. Next: causes 3–5 (milestone 2).

## Round-4 milestone 2: causes 3–5

Commits over `1198fc24`, baseline `3cc12c6f`: **77c63039** (reuse invariant and
pool-poison mode), **5d8b951a** (cause 3), **b72bd74f** (cause 4), **3d028455**,
**dc507107** (cause 5), **832328a4** (repair), **d6633cd1** (x86 order),
**f7057a51** (poison checks out of release builds).

**Cause 3.** A slot-backed body that cannot observe its retained chain enters
without it while `ScopeCursor::resolves_globally` holds (no with, no
suspendable-owner holder, no holder binding a free or captured name, no other
fixed cell of one), cached per (function, holder epoch); every holder add/delete
bumps the epoch. Every function with a retained chain needs the proof, which also
closes a pre-existing gap: `new C()` over an eval scope ignored an eval-declared
name. **Cause 4.** Leaf purity is a plan bit; typed-loop closed-form calls build a
Context only for natives. **Cause 5.** The VM loop is one body instantiated as a
checking and a fast instance; writers of scope tracking/authority set
`FrameState::slow_flags`, the fast instance tests it after out-of-line ops only.
Global-code var stores use the slot's realm cell (realm-table generation, else one
lookup); wide plain global stores keep a per-site realm-cell memo.
**Repair (832328a4).** aea492db's typed-loop sink seeded its register from an
empty fallback slot: a loop admitted after `if (i == 4) g = 7` created `g` wrote
`undefined` back. Regression tests: `typed_loop/global_binding_tests.rs`,
`tests/global_cell_routes.rs`, `tests/extensible_scope_closures.rs`.

Reproducers, local arm64 instructions vs base at f7057a51 (f609c0ae in brackets):
upw0/upw1/upw4 1.149/1.149/1.174 [1.164/1.168/1.177]; call-implicit/call-var
global write 1.318/1.058 [1.500/1.319]; vm-ops/-local/-plain 1.045/1.280/1.009
[1.190/1.279/1.074]; closed-form-call 1.032 [1.187]; eval-closure/-parent-closure
1.432/1.069 [1.521/2.959]; validate-like 0.969; bitand/partial-global 1.019/1.016.
Public cases: date-format-xparb 1.048, string-tagcloud 1.026 (tier parity).

| Public86 job (vs 3cc12c6f) | broad | development | external | sentinel |
| --- | ---: | ---: | ---: | ---: |
| `20261101T115125Z-t077-s3b-r4-m2` (d6633cd1) | 1.019 / 1.034 | 1.049 / 1.025 | 1.028 / 1.032 | 0.983 / 1.033 |
| `20261101T120529Z-t077-s3b-r4-m2b` (f7057a51) | 1.037 / 1.031 | 1.034 / 1.011 | 1.025 / 1.026 | 0.976 / 1.032 |

Wall / instruction lane geomeans; both 430/430 valid, budget met, likely loss.
Lanes stay above 1.00, so the final requalification did not run (lead rule).
Remaining cost by class (m2b instructions):

- Per-call activation binding, about 130–150 instructions per call: slot
  rebinding (~40), owner binding (~15), out-of-line frame push (~30), release
  checks (~25), Context rebinding at both depth changes (~20), receiver reads
  through the owner. broad top_level/plain/reordered/two-args calls
  1.12/1.08/1.08/1.08, method_call 1.06, captured_read 1.10, sentinel
  capturing/polymorphic calls 1.07/1.06, binary-trees and
  raytrace-public-class-fields 1.14, hash-map 1.06, lodash 1.07. A same-function
  slot fast path lost (lodash +2.5%). Removing the cost needs on-demand activation
  binding in the register tier, beyond the S3b plan.
- Captured writes: `captured_write` 1.16 adds a checked `BindingCell::assign` per
  store (TDZ/metadata/strictness, ~125 instructions); a compile-time unchecked-set
  policy is not implemented.
- Global code with configurable globals keeps the checking loop and per-site
  events (call-implicit-global-write 1.32); eval-function locals read through
  cells (vm-ops-local 1.28; date-format-tofte 1.08, xparb 1.04).
- Wall-only: lodash 1.18, string-unpack-code 1.17 (instructions 1.07/1.04),
  branch_arithmetic 1.48 (1.02) after the order was generated at dc507107.
- Diffuse 1–4% without call-count differences (partial-sums 1.09, base64 1.07,
  nbody, crypto, 3d): typed-loop/property/string codegen; not attributed by counts.

Checks at f7057a51 unless noted: fast gate at every commit (2,722 runtime + curated
Test262), perf-counters 2,771, runtime tests under binding/scope/activation poison
2,722, 17 reference-sensitive Test262 prefixes 3,779/3,779 cached and IC-off;
at dc507107: compare-qjs 318/318, differential seeds 0–299 × three profiles,
900 programs tag-identical to the final qualification with no mode disagreement.
Depth maxima forEach 936, map 927, reduce 1,026, call 3,449, apply 2,335,
reflect_apply 2,492, yield_star 820, plain 4,096 (all ≥ base). No timing ran locally.

## Round-4 milestone 3: global-store repair and final requalification

The 689efca2 order (job `20261101T122522Z-t077-s3b-final2-order`) froze runtime
f7057a51. Its requalification stopped on the lead's order when an independent
re-review of 689efca2 confirmed four regressions in mechanism 1; only complete
records are kept ([689efca2 public86](T077-s3b-final2-public86/README.md)). Commits:
**0148b2eb** (repair), **a84252bd** (differential profile), **8abd4c69** (x86 order,
[job `20261101T140029Z-t077-s3b-final3-order`](T077-s3b-final3-order/README.md), 468
symbols). Frozen runtime **0148b2eb**, frozen sha **8abd4c69**.

**Repair (correctness defect).** An elided bound start (aea492db) and every
compound/update site stored through the old StoreGlobal paths, which looked only
at the global object's own properties and the frame's mirror slots: inherited
setters and read-only properties were bypassed by an own data property, strict
code threw ReferenceError for a name only the chain bound (also after an RHS
delete, and through a Proxy in the chain), and a strict store kept writing a
property after another script, or its own RHS, declared a global lexical
(F1–F3). Since f609c0ae a strict function created by eval or Function also lost
its write to a script `let` (F4/F4b): the realm-table insert cannot assign a
lexical. `binding::set_bound_global` is now the one store for a reference bound
to the global environment: the lexical at store time, else HasProperty over the
whole chain and [[Set]], then the realm mirror. Only a realm var cell proven by
table generation to name an own writable data property, and own-data writes,
skip the walk; mirror slots are refreshed from the canonical binding; the
register tier creates a global only when the ordinary chain lacks the name.
The pre-existing P1 (later lexical ignored after a strict property write) and P2
(strict `new Function` write to a script `let`) and a lexical string append that
also wrote a shadowed property are fixed by the same store. Tests:
`tests/global_store_semantics.rs` (once and hot), NG fixtures
`global-store-inherited-setter.js`, `global-store-declarative-first.mjs`,
`global-store-object-record.mjs`; the differential gains the review's genA as
profile `global-store` (byte-identical programs per seed).

**Cost of the repair**, local arm64 instructions, 689efca2 runtime → 0148b2eb, one
run per public workload, median of three per reproducer
([before/after](T077-s3b-final3-owner/fix-cost/)): 55 external+development
workloads geomean **1.0005** (development 0.9997, external 1.0006; access-nsieve
1.012, date-format-xparb 1.008, math-cordic 1.006 the largest, string-unpack-code
0.995 the smallest). Reproducers: call-global-var-write 1.012, call-implicit-
global-write 1.012, eval-parent-closure 1.011, global-lexical-write 1.006,
vm-ops 1.005, others within ±0.3%; strict-global-var-write 0.782 and
compound-global-write 0.838 (realm-cell/own-data paths replace the old strict and
compound fallbacks). On x86, public86 per-case instructions move 1.0030 against
the 689efca2 job (development 1.0074, external 1.0033, broad 1.0015, sentinel 1.0000),
repair plus new order.

**Checks at 8abd4c69** ([owner receipt](T077-s3b-final3-owner/README.md)):
full gate exit 0 (2,728 runtime tests, 5,174 curated Test262);
agents 2,739, perf-counters 2,777; stress 2,728 in each of seven modes
(verify-bindings, forced exposure, scope poison, activation poison, IC verify,
combined, combined-all) and combined IC-off 2,686 with the unchanged 42 controls;
compare-qjs 319 scripts × 9 modes and 10 module files × 9 modes match NG; public
correctness 351/351; complete exact Test262 cached and IC-off 42,672/42,672 each,
zero gaps, case maps byte-identical to the prior complete map, burndown rows at
0148b2eb (cached) and 8abd4c69 (IC-off); counter maps reproduce twice, base maps
unchanged, retired CallEnv/snapshot/apply-back events zero. Differential 8,000
programs: binding/binding-simple/receiver 6,000 tag-identical to 7b3ed258 (742
triaged programs identical, no new REGR, no mode disagreement); global-store MATCH
527, FIXED 1,445, CHANGED 39, REGR 35 (7b3ed258 runtime on the same seeds: REGR 68,
CHANGED 419, PRE 38); 134 of 181 non-NG observations equal that runtime's, the 47
others (14 programs, 3 REGR) and every inspected shared one are the
unresolvable-at-start rule (strict ReferenceError, sloppy global-object property
beside an RHS-declared lexical; NG and V8 re-resolve) and repaired values. Depth:
release maxima all above base (call 3,234→3,449); **debug call/apply/Reflect.apply
fall below base (461→437, 372→366, 373→367)** since f609c0ae, whose
`wide::activation::run` debug frame grew 13,776→15,376 bytes; not changed (no
semantic effect, release higher, source frozen), lead decision.

| Public86 job (vs 3cc12c6f) | broad | development | external | sentinel | all 86 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `20261101T123235Z-t077-s3b-final2` (689efca2) | 1.016 / 1.031 | 1.029 / 1.012 | 1.030 / 1.027 | 0.980 / 1.032 | 1.022 / 1.027 |
| `20261101T140707Z-t077-s3b-final3` (8abd4c69) | 1.017 / 1.033 | 1.034 / 1.019 | 1.026 / 1.030 | 0.991 / 1.032 | 1.021 / 1.030 |

Wall / instruction geomeans; both 430/430 valid, budget met, likely loss.

**Standing: ready for lead acceptance as an enabling stage** (lead acceptance
bound: overall regression about 3% or less). S3c owns activation ownership and
these recorded obligations, not started here:

1. Per-call activation binding, about 130–150 instructions per call (slot
   rebinding ~40, owner binding ~15, out-of-line frame push ~30, release checks
   ~25, Context rebinding at both depth changes ~20, receiver reads through the
   owner); upw0/upw1/upw4 1.149/1.149/1.174, broad calls 1.08–1.12.
2. Checked captured-cell writes, about 125 instructions per write
   (`BindingCell::assign` TDZ/metadata/strictness); captured_write 1.16–1.19.
3. Global-code reference hooks (call-implicit-global-write 1.32) and eval-local
   cell reads (vm-ops-local 1.28, date-format-tofte 1.08).

Residual classes, recorded without an owner action here: 4. wall-only layout
gaps (lodash 1.19 wall / 1.09 instructions, string-unpack-code 1.17 / 1.04);
5. diffuse 1–4% typed-loop/property/string codegen (partial-sums, base64, nbody,
crypto, 3d) without call-count differences.
