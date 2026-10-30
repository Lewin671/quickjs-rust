# T077 S3: exhaustive Op to Code inventory

Source: `dd0905b2cb0236c38619089a27e8342b575dda61`, `bytecode/ir.rs::Op`.
Paths in this inventory are relative to `crates/qjs-runtime/src/`.
Target mnemonics below are proposed semantic instructions, not implemented APIs.
Every variant is named exactly once; `Wide` is an encoding prefix, not an executor.

| Existing Op variants | Proposed Code instructions | Required semantics / metadata |
| --- | --- | --- |
| `LoadConst` | Const dst,pool | Constant index checked; owning value copied only when live. |
| `LoadLocal`, `LoadLocalOrUndefined` | ReadBinding dst,route / ReadBindingOrUndefined | BindingPlan selects local/cell/import/realm; missing differs from TDZ. |
| `LoadNewTarget` | LoadContext dst,NewTarget | Lexical owner, not a magic name. |
| `StoreLocal`, `AssignLocal`, `ClearLocal` | InitBinding / AssignBinding / ClearBinding | Keep initialization distinct from assignment, mutability and TDZ; clearing ends lexical lifetime. |
| `DefineGlobalVar`, `LoadGlobal`, `StoreGlobalStrict`, `StoreGlobalSloppy`, `StoreLocalOrGlobalSloppy`, `TypeofGlobal` | DeclareRealm / LoadRealm / StoreRealm / ResolveSloppyStore / TypeofName | Global object versus declarative cells; sloppy unresolved creation and strict failures remain distinct. |
| `AppendStringLiteralLocal`, `AppendStringLiteralGlobal` | AppendStringBinding route,const,result-mode | Retain unique-string append ownership; result may be discarded; no tier recognizer. |
| `EnterWith`, `ExitWith` | PushObjectScope / PopScope | Scope lease keeps object and unscopables lookup order. |
| `LoadIdentWith`, `ResolveIdentWith`, `LoadResolvedIdentWith`, `StoreIdentWith`, `StoreResolvedIdentWith`, `TypeofIdentWith` | ResolveName / GetReference / PutReference / TypeofReference | Reference token keeps chosen base across RHS callbacks; no second resolution. |
| `Pop`, `Dup` | DropTemp / CopyTemp (or no emitted word) | Liveness controls Drop/move; observable bindings cannot be consumed as dead temps. |
| `NewArray`, `NewTemplateObject`, `NewObjectLiteral`, `NewObjectDataLiteral` | NewArray descriptor / GetTemplate site / NewObject / NewDataObject layout-site | Cold literal/spread metadata in pools; S1 creation API, per-realm template identity. |
| `SetComputedFunctionName`, `DefineObjectProperty`, `CopyObjectSpread` | SetFunctionName / DefineProperty / CopyDataProperties | Key conversion, getter evaluation and descriptor order are resumable effects. |
| `EnumerateKeys`, `ForInKeyIsEnumerable` | EnumerateKeys site / CheckForInKey | S2 dependencies and live recheck; no copied value cache. |
| `GetPropNamed`, `GetPropIndex`, `GetProp` | GetNamed site / GetIndex / GetKey | Explicit dst,receiver,key; fused receiver index decoded during lowering; hooks use continuation. |
| `SetProp`, `SetPropNamed`, `SetPropIndex` | SetKey / SetNamed site / SetIndex | Strictness preserved; expression result separate from overwritten temp; S2 sites. |
| `GetPrivate`, `SetPrivate`, `PrivateIn` | GetPrivate / SetPrivate / HasPrivate | Private-environment metadata and brand checks, no public property shortcut. |
| `DeleteProp`, `DeleteIdent`, `DeleteIdentWith` | DeleteProperty / DeleteBinding / DeleteReference | Strict errors, deletable eval cells, with reference and unresolved names. |
| `RequireObjectCoercible`, `RequireCallable` | CheckCoercible / CheckCallable | Keep throwing position and no premature effects. |
| `GetIterator`, `GetAsyncIterator`, `AsyncIteratorComplete`, `IteratorStep`, `IteratorRest`, `ObjectRestExcluding`, `IteratorClose` | GetIterator / GetAsyncIterator / IteratorComplete / IteratorStep / IteratorRest / ObjectRest / IteratorClose | Each has explicit registers, done flag and cold semantic step; close preserves pending completion and swallow rule. |
| `Call`, `CallResolved`, `CallResolvedGuardedMathUnary` | Call dst,callee,receiver,args,site | Receiver already resolved; guarded Math becomes primitive guard plus same direct NativeEntry, not a second dispatcher. |
| `CallDirectEval`, `CallDirectEvalSpread` | EvalCall / EvalCallSpread | Live intrinsic identity and direct-call syntax determine direct eval; rebound callee uses ordinary Call. |
| `CallSpread`, `CallResolvedSpread`, `NewSpread`, `SuperCallSpread` | CollectArguments + Call / Construct / SuperConstruct | Observable iteration once; bounded reusable staging, abrupt iterator close. |
| `New`, `SuperCall` | Construct / SuperConstruct | Explicit new.target, allocation/derived-this owner and completion policy. |
| `NewFunction`, `NewClass` | CreateClosure template / DefineClass descriptor | Indexed captures, dynamic leases, private/home context; computed keys/static blocks suspend enclosing frame. |
| `SuperGet`, `SuperReference`, `SuperGetComputed`, `SuperSet`, `SuperSetComputed`, `SuperMethod`, `SuperMethodComputed` | ResolveSuper / GetSuper / PutSuper / ResolveSuperMethod | Keep receiver and lookup base distinct, resolve before key/RHS, retain method receiver. |
| `Typeof`, `ToString`, `ToPropertyKey`, `ToPropertyKeyForAccess`, `ToNumeric`, `Unary`, `Update`, `Binary` | Typeof / ToString / ToKey / ToAccessKey / ToNumeric / Unary / Update / Binary | General primitive checks and resumable coercion; preserve numeric-index fast form, BigInt and -0. |
| `Jump`, `JumpIfFalse`, `JumpIfTrue`, `JumpIfNotNullish` | Jump / BranchFalse / BranchTrue / BranchNotNullish | Targets relocate to instruction-start word PCs, complete init/liveness joins. |
| `AbruptJump`, `EnterTry`, `ExitTry`, `EndFinally`, `DiscardPendingAbrupt`, `Return`, `Throw`, `ThrowReferenceError` | SetCompletion + DispatchHandler / LeaveHandler / EndFinally / ClearCompletion / Return / Throw / ThrowReferenceError | Handler table includes cleanup/scope depths; abrupt continuation can replace pending completion. |
| `FreshIterationScope` | FreshCells binding-list | New identity each iteration, copies current binding values exactly once. |
| `EnterDisposableScope`, `RegisterDisposable`, `RegisterAsyncDisposable`, `DisposeScope` | EnterDisposeScope / AddDisposable / AddAsyncDisposable / DisposeScope | LIFO, once-only method resolution, SuppressedError precedence, await continuation. |
| `FunctionPrologueEnd`, `Yield`, `Await`, `YieldDelegate` | PrologueEnd / SuspendYield / SuspendAwait / Delegate | Parameter errors retain sync/async distinction; resume mode and delegated iterator state are owned. |
| `ImportCall`, `ImportMeta` | ImportCall dst,options-mode / ImportMeta dst | Referrer and realm from typed context; host job ordering and promise rejection preserved. |
| `InitVirtualObject`, `LoadVirtualValue`, `StoreVirtualValue`, `LoadVirtualLength`, `GuardVirtualObject`, `InitVirtualConstants`, `LoadVirtualBinary`, `BinaryAssignLocals`, `IncrementLocal`, `CopyLocal`, `CompareLocalsJumpFalse`, `InitVirtualFunction`, `CallVirtualFunction`, `LoadVirtualNumber` | NewObject/NewArray/CreateClosure + ordinary register instructions; scalar replacement in S4 | These are lowerer-only Op variants. S3e removes executable virtual streams/selector/bank; expand their source semantics before lowering, then S4 optimizes one Code. No LegacyOp or hidden allocation_ip reference survives. |

Coverage: **120 variants**, checked against the enum at the source revision.

## Universal lowering and verifier obligations

The frontend keeps its binding analysis. Ephemeral stack IR may feed the one
builder, but after S3e neither executable/cached Op nor a whole-program fallback
exists. Map stack depth to temporary register identities on every CFG edge;
normalize differing incoming stack locations with edge copies, including catch
entry, finally completion and resume values. Locals remain fixed low indices.
BindingPlan owns local/cell/realm/import/dynamic routes, never copied values.

Code owns ABC/ABx u32 words plus canonical Wide operands; all variable-sized
data above lives in constant/site/template/handler tables. Decode once per op.
Verifier rejects malformed lengths, extension-word targets, invalid ranges,
uninitialized temp use, invalid route writes and incompatible CFG state maps.
Generate coverage from exhaustive matching over Op with no wildcard; test every
row, >255 registers, large constants/jumps, handler edges and suspension edges.

Old accelerator PCs are Op indices. During S3e compile typed/mutation analysis
from ephemeral IR, relocate every entry/recovery PC and slot map through the
builder, validate that exits target Code instruction starts, then release IR.
Do not retain source Op via CallVirtualFunction or as a deoptimization oracle.

## Landing and ownership

See [stage plan](T077-s3-plan.md). One compiler/builder owner owns `ir.rs`,
`compiler*.rs`, encoding and verifier; semantic owners return instruction
requirements and tests, not competing opcode enums. S3a–d leave this mapping
as design input; S3e replaces stack/compact/wide/leaf execution together.
S1 layout creation and S2 site APIs are consumed through their public runtime
interfaces; no edit of S1-owned property/value/layout paths is scheduled here.
