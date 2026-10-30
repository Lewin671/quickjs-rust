# T077 S1: allocating property-operation error boundary

Status: lead decision requested; no fallibility/API correction implemented.
The explicit-owner preparation is independent of this decision; the ordinary
store cutover is held at this boundary under the implementation brief's rule:
"Stop and report instead if the correction would change the stage's scope or
the API that S2/S3 depend on."

## Concrete mismatch

The approved domain allocates monotonic, non-reused u32 KeyIds and returns
`AtomDomainExhausted`, converted to a `RangeError` RuntimeError. The configured
exhaustion tests exercise the last ID and prohibit wrapping. Every new ordinary
string or symbol definition must obtain a lease before changing NamedStore.
Reading an existing or absent property need not allocate an ID; `find` and
`find_symbol` provide that distinction without a fallback property store.

Current ordinary APIs do not have an error channel:

- `ObjectRef::{set_shared_key,define_property,define_symbol_property}` return `()`.
- `OrderedDataPropertyBuilder::{insert,insert_property}` return `()`; bulk
  literal/HashMap constructors return ObjectRef.
- `Function::new_native` creates an ordinary constructor prototype and defines
  its `constructor` property, but returns Function.
- Builtin installers return `()`; `new_module_realm` returns Realm and
  `ModuleGraph::new` returns Self. Failure from nested key creation cannot
  reach the existing script/module RuntimeError result through those APIs.
- `properties[_for]` currently materialize pending String indices by defining
  properties. They are called beneath `Option<Property>`, bool and own-slot
  probe APIs, so interning an index here would introduce a fallible operation
  under a currently pure result type.

Using expect/panic, discarding exhaustion, returning a miss, reusing an ID,
allocating another domain or keeping a text fallback would violate the approved
contract. Merely giving constructors a domain does not settle the error ABI.

## Recommended lead amendment

Allow S1 to make allocating construction/mutation/materialization paths return
`Result<_, RuntimeError>` and propagate it through native constructor creation,
builtin installation and module-realm/graph initialization. Empty construction
can remain infallible because it interns no key. Existing borrowed-lease layout
APIs and S2's one-authority contract remain intact.

Keep own-slot probes pure: return NeedsSlowPath/Exotic for any operation needing
materialization; the semantic path must retain the error. String virtual-index
reads may avoid materialization entirely where the exact descriptor can be
computed without observable effects. Publish no partially initialized mutation
when key creation fails. Add exhaustion discriminators at these boundaries,
including a deferred String wrapper, rather than testing only AtomDomain.

An alternative is to keep selected fixed-key host constructors infallible by
passing strongly retained, prelinked host-key leases instead of interning text
inside them. That needs an approved owner/lifetime contract, including exhausted
domains whose original realm owner has died; it cannot by itself cover arbitrary
dynamic definitions or deferred String materialization.

The lead must confirm the cross-stage error/owner contract before the cutover. No
correctness waiver, legacy store fallback or scope reduction is proposed.
