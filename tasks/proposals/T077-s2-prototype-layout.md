# T077 S2 round-1 prototype layout correction

Lead feedback C replaces the initial shared-layout subscription design. Base:
`c2d65271` after the requested S1 merge and queued read/context repairs. Parts
C-E are one staged architecture migration; no old store or alternative engine.

## Target and invariants

- Shared immutable layouts include prototype identity and receiver/virtual-hook
  flavor. Roots/transitions are per prototype identity; shared guards compare
  allocation identity only. Dictionaries alone use nonzero versions and publish
  a fresh allocation before version rollover. Creation routes converge to the
  same root/transitions regardless of literal/constructor/incremental history.
- A layout retains weak prototype identity, while the receiver's named authority
  owns the actual strong prototype edge. This implements prototype-sensitive
  maps without making byte-retained shapes pin JavaScript prototypes or realms.
  Changing the prototype re-roots the object's complete named layout atomically.
- Objects used as prototypes get private dictionary layouts and one prototype
  state, with a replaceable validity cell and weak prototype-user registry.
  Structural edits invalidate and cascade iteratively through current users;
  value-only writes do not. Dead/stale users are pruned amortized; there is no
  subscriber cap and no per-IC registration. Ordinary edits invalidate nothing.
- Arrays/functions keep a lazy named authority: an empty canonical layout plus
  explicit prototype until properties need value slots. Probing never allocates
  array cold data or materializes unrelated function defaults. The full variant
  is the same NamedStore, not a second named authority. Virtual function-state
  changes transition the flavor so hook/data/missing proofs cannot alias.
- Realm intrinsics are immutable creation inputs. A shared AtomDomain is not a
  realm identity: native/array creation APIs receive realm/prototype information,
  and bootstrap allocates root intrinsics before installing methods. No prototype
  lookup consults mutable Function/Array bindings. Array prototype identity is
  the underlying live array, not a freshly allocated contextual wrapper.
- IC outcomes include own/inherited data, accessors, missing and explicit virtual
  hooks. Own data uses a layout compare plus live slot load; inherited entries
  also use the first prototype's validity cell and a weak holder/slot. Getters
  execute once with the original receiver after borrows end. Debug verification
  compares complete pure outcomes and fails on any incompatible outcome.
- Debug switches are immutable per runtime-group/domain configuration, read once
  when that owner is created. Entry admission binds that configuration to its
  domain/layout; no mutable process-global engine state or per-entry getenv.

## Migration and verification

1. Prototype identity/roots/private mode cells and lazy authority; explicit realm
   intrinsic creation wiring, preserving all S1 shared-tail/retention repairs.
2. Port ICs, enumeration/creation/global-slot consumers; remove old subscribers,
   epochs on shared guards, implicit prototype resolution and side-effecting
   verifier paths. Keep mono/four-way/mega; retire dead guards as stale.
3. Finish D memory/verifier scenarios, then E generic semantics and iterative
   named-property teardown. Add a differential fixture for every E family.
4. F full/NG/IC-off/debug/exact gates, burndown, development + review counters,
   then authorized iMac quick triage against `dd0905b2`; no held-out diagnostics.

Proposed hot sizes (assert and execute on both production targets): adding an
explicit prototype owner to NamedStore may grow it from 48 to 56 bytes, while
removing the separate ordinary prototype cell shrinks ObjectData from 96 to
80 bytes. A lazy authority can keep packed arrays around 88 bytes and functions
around 304, avoiding their former probe-time cold/name-slot allocations. These
are estimates until measured; correctness and whole-portfolio payoff govern.
No dependency, crate, unsafe, public error ABI, AST or binding-model change.

## Payoff repair after the first triage

Job `20261030T135556Z-t077-s2-r1-triage` misses payoff versus dd0905b2.
Profiles `20261030T140601Z-t077-s2-r1-property-profiles` identify descriptor-size
cache return frames on named hits and repeated atom recovery on computed keys.
Keep data-hit ABI separate from cold descriptor outcomes and compare own poly
ways under one store borrow. Memoize one domain-checked KeyLease in shared
JsString metadata, using its existing metadata RefCell; reads only find live
keys, mutations/COW clear the memo, and foreign domains replace it. The memo
pins a lease, never a domain/receiver. This adds 8 logical header bytes without
changing Value/JsString handle sizes; measure the whole development set again.
All semantic hooks and generic fallbacks remain the same NamedStore authority.

The first repair pays back dynamic/churn (0.7819/0.9425) and matter-js (0.9831),
but prototype calls remain 1.3505. Profile job
`20261030T143230Z-t077-s2-r1-payoff-profiles` still shows the untyped typed-loop
read entering the descriptor handler and an out-of-line generic with_named
closure in own poly hits. Route all read consumers through the small ABI;
borrow the actual first-prototype edge under the receiver guard to load a direct
holder, retaining the same validity generation/slot proof. Force only the small
authority-borrow wrappers inline and split dense-array's empty-cold check from
its descriptor-index formatting body. Prototype identity comparisons use raw
borrowed identities, never temporary Weak handles. No cache budget/semantics change.
