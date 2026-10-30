# T077 S3b: frozen Context and binding implementation brief

Design frozen at `71294a20864662808216f874635dc8fb0473693e`, 2026-10-30;
documents only, no implementation or performance acceptance claimed. Audience:
the S3b implementation owner after accepted S2. [T077](../T077-core-rebuild.md)
owns acceptance; the six [T074 decisions](T077-s3-eval-scope.md) are approved.

| Part | Frozen authority in this brief |
| --- | --- |
| Context and S2 boundary | This file |
| BindingPlan, descriptors, leases, exposure | [Binding design](T077-s3b-bindings.md) |
| Every CallEnv method and consumer family | [Counted port inventory](T077-s3b-inventory.md) |
| Buildable commit series, deletions, owners, lead gates | [Migration](T077-s3b-migration.md) |
| T074 1–13, T071, stress and exit | [Verification matrix](T077-s3b-verification.md) |

## Source and prerequisite pins

This is the S3a-integrated design base, **not** the eventual S3b runtime base.
Before code, lead records the accepted S2 integration SHA and this brief's SHA
in T077, recounts the inventory delta, and binds the performance endpoints.
No runtime work or measurement is authorized by this documentation submission.

S2 interface audited read-only with `git show` at
`26fee93031a664d952d0541b838ca7f8453097d6` on
`agent/t077-s2-named-stores/codex`, `docs/design/core-rebuild-objects.md`.
Assumption: accepted S2 retains that contract, including prototype-in-layout,
weak validity generations and one NamedStore per receiver; the pin is an input,
not a claim that this unaccepted branch is main. Lead resolves any API delta
before the first port. S3b never reads NamedStore internals to avoid this gate.

## Types and ownership

Internal types only, std/Rc/RefCell, no new crate, dependency, unsafe, FFI or Rust
async. Names below specify the target API; split modules by responsibility.

```rust
struct RuntimeServices {
    group: RuntimeGroup,                 // AtomDomain, realm registry, job services
    host: HostServices,                  // module resolver/host; no lexical names
    #[cfg(feature = "agents")] agent: Option<AgentContextRef>,
}
struct CreationContext {
    realm: Realm, module: Option<Rc<ModuleContext>>,
    referrer: ScriptOrModule, private: Option<PrivateEnvironment>,
    global: GlobalSelection,             // Default / Override(object) / Disabled
}
struct ActivationContext {
    creation: Rc<CreationContext>,       // function/root-owned; shared, not copied maps
    call: LexicalCallContext,            // own or inherited typed owner
    scopes: ScopeCursor,                // optional escaping lease + active scope IDs
    variable: VariableEnvIdentity,       // fresh logical identity for each ordinary call
    mode: ExecutionMode, strict: bool,  // script/module/function/eval/field/static/key
}
struct Context<'a> {
    runtime: &'a mut RuntimeServices,
    activation: &'a mut ActivationContext,
    native: Option<NativeSite<'a>>,      // borrowed callee/info + construct target source
}
type NativeEntry = fn(&mut Context<'_>, CallInfo<'_>, &[Value])
    -> Result<Value, RuntimeError>;
```

`RuntimeGroup` is the existing communicating-realm/AtomDomain ownership boundary;
it does not acquire a general executor or GC in S3b. Jobs and host references
remain their current owners' services, not duplicated queues. `ModuleContext`
retains canonical import/export cells and module identity, never settled values.
Global lexical metadata moves to a canonical realm declarative cell store;
the old global lexical **value** projection is removed.

`LexicalCallContext = Inline(CallOwner) | Shared(Rc<CallOwner>) | None`.
CallOwner has `ThisBinding::{Initialized(Value),Derived(BindingCell)}`,
`new_target: Option<Value>`, home object, super constructor, active constructor,
and field/static restrictions. At first arrow/eval retention, move the inline
owner into Rc and redirect the declaring activation to Shared; never mirror it.
Ordinary calls allocate no Rc owner unless lexical sharing is needed. Module/script
receiver is initialized undefined/global-this respectively, never derived TDZ.
`arguments` is absent from this type: it is a binding in the scope plan.
An inherited owner cannot overwrite an inner lexical arguments declaration.

`VariableEnvIdentity` is a fresh activation-local token even without a lease;
no map/Rc allocation is required for a slot-only function. On exposure its own
holder acquires that identity and survives with its lease. Root/module/eval
builders select their specification environment; they do not mimic a function.

## Construction and borrowing

| Entry | Construction / authority |
| --- | --- |
| Host script/module | Root builder selects realm/global/module cells and referrer once; Vm owns ActivationContext beside current locals, with a mutable RuntimeServices borrow. |
| Ordinary JS call, including leaf/compact/wide | `enter_function(function, plan, receiver, target, incoming)` builds only typed activation fields and positional routes; fresh variable identity. Never inherit caller lexicals by copying. |
| Arrow | Function captures its lexical owner and needed indexed cells/lease at creation; call uses its own parameter scope, inherited this/new.target/home. |
| Direct eval | `enter_direct_eval(site, compiled_plan)` borrows the active chain and explicit parse context; sloppy declarations target the selected own variable holder. Strict eval adds a new variable/declarative scope. |
| Indirect eval / Function | Select callee realm global chain/creation context, no caller lexical lease; preserve explicit marked-global override/Disabled semantics as typed data. |
| Native | Reborrow current services/activation into a stack Context with NativeSite; **zero** binding container, activation or realm clone merely to call a builtin. |
| Callback | Release all storage/cell borrows; reborrow services into a child JS activation from the callback's creation context. Parent typed owner and operand owner stay parked. |
| Suspend/resume | Current snapshot owner retains ActivationContext, leases and cells by identity; borrow a Context again on resume. S3c later removes snapshot/block reconstruction. |
| No-context conversion/JSON API | Explicit host service root once at API boundary; internal pure primitives take no Context. Observable hooks receive a real selected realm Context, never an empty-env factory. |

A Context is never Clone, stored on a Function, or retained by a job/continuation.
Functions retain CreationContext, lexical owner and cells/ScopeLease; native
closures retain typed capture structs (promise reaction, regexp error, iterator,
etc.). Completion carries result/throw and genuine semantic state, no env to
apply back. Current Vm/compact/wide frames own activation fields; split their
storage and service borrows to construct Context only for an effectful segment.
Do not hold a mutable Vm/Executor RefCell guard while calling JS.

`NativeSite` borrows S3a's CallInfo and a target source supplied by construct
dispatch; `ctx.realm()` lazily selects the native callee creation realm,
`ctx.caller()` identifies lexical direct-eval/host caller state, and
`ctx.new_target()` queries the actual construct target only on demand.
The target is distinct from callee, including Reflect.construct/bound forwarding.
Native call new.target is absent even inside a constructing caller. Derive eval
parse context from caller strictness, typed owner and field/super restrictions.
`with_native` reborrows into a child facade; return/throw drops that facade, so
no eager realm/new.target clones, guard Drop aliasing, or manual restoration is
needed. JS callbacks build their own selected context; they cannot mutate the
parked parent's call owner. No native-specific depth limit is introduced.

S3a ABI migration: change the service parameter in `native.rs::NativeEntry`,
`call_native_function`, all bound entry functions and their helper signatures
from CallEnv to Context. Keep CallInfo's borrowed callee/receiver/construct bool,
argument slices, operation bind match, guard order and argument-owner detachment.
Port capability calls via the inventory; no Context→CallEnv adapter or dual
native ABI. Maintain ordinary JS fixed-arity admission order and callback leaf
shortcut; no new native-kind admission load in the JS→JS path.

## Property and other services

Expose `ctx.{realm,atoms,intrinsics,global_env,module,host,agent,private_env}` and
`ctx.{call,construct,enqueue_job,eval}` through existing semantic modules.
These effectful methods may recurse through current JS entry only until S3d;
S3b does not invent a second executor or claim explicit-stack completion.

S2 semantic calls take Context: `to_key(ctx, Value) -> Result<KeyLease>`,
`get(ctx, receiver, key)`, `set(ctx, base, key, value, receiver, strict)`,
`get_own_descriptor`, `define_own`, `delete_own`, `own_keys`, `set_prototype`.
Callback-capable variants take ctx even if the present fast path is pure.
Keys retain domain/lease identity; linked operands remain prelinked. Release
every store/layout/cell borrow before coercion/getter/setter/Proxy/callback or
lazy installation. Keep exotic hooks and global-object/cell synchronization;
no cached property Values or new invalidation system in Context.
