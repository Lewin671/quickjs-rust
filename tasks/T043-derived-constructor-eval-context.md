# T043: Derived-constructor direct-eval context

- Status: active.
- Implementation: none; two baseline semantic gaps are reproduced.
- Verified at: exact `06f0ecd3`, 2026-10-12, freshly compiled local debug CLI
  versus clean fixed NG `f7830186`. These are correctness runs, not timings.
- Evidence: minimal programs and outputs below. Raw scripts/JSON are workspace-
  local under `outputs/quickjs-performance-20261012/base06-correctness`, not
  retained in this repository. Base executable SHA
  `622ec1add4f750cd14b7ef762b8072eaaf11b3260c2ea84afd8f5eddbb62dbed`.
- Unresolved: propagation of implicit `this` initialization and the derived
  `this` TDZ through the existing native eval route; introduction revision.
- Next action: trace the original eval/frame/constructor context, add focused
  differential coverage and fix separately from T042's read-only ownership unit.

## Reproductions

```js
class B { constructor() { this.x = 4; } }
class D extends B { constructor() { eval('super()'); } }
[new D().x, new D().x].join(',');
```

NG returns `4,4`; the exact baseline throws
`ReferenceError: super constructor may only be called once`.

```js
class B {}
class D extends B {
  constructor() {
    var ok = [];
    try { eval('this'); } catch (e) { ok.push(e instanceof ReferenceError); }
    try { eval('this'); } catch (e) { ok.push(e instanceof ReferenceError); }
    super();
    this.ok = ok.join(',');
  }
}
[new D().ok, new D().ok].join('|');
```

NG returns `true,true|true,true`; the exact baseline returns `|`. Thus both
cold and warmed eval fail to preserve the pre-super `this` TDZ. A fuller
two-instance event probe records first and repeated `super()` completions:
baseline `first:ReferenceError|second:ReferenceError` per instance, NG
`first:ok|second:ReferenceError`. Counting only allocations or constructor
side effects hides this difference and is not adequate coverage.

These failures occur before T042's new implementation. That exact-base
reproduction does not identify their original introducing commit; do not
ascribe them to an earlier architectural change without a bisect.

## Scope and boundary

Trace `global.rs` native eval, `bytecode/vm_call.rs` caller projection and
writeback, `vm_frame_init`/binding/`this` handling and derived construction.
Preserve per-activation `this` identity, shared cells, single-super rules,
callback effects and exceptions. T042 conservatively excludes direct
SuperCall/Spread effects and uninitialized derived-this contexts; its ordinary
read-only callback coverage must still preserve shared-cell effects.

The configured Test262 snapshot's zero-gap result does not establish these
untested programs are correct. Add cases that assert each completion and `this`
state, not merely a final allocation count, then run focused Test262 and NG
comparisons plus the full correctness gate.
