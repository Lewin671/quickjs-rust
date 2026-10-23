# T069: Deferred class-key call-context repair

- Status: historical-campaign; requested repair and verification complete, lead decision pending.
- Implementation: receiver repair `e38e85fc`; corrected comparison fixture `6c512032`.
- Verified at: runtime `6c512032`, 2026-10-23: final full gate, 16 focused tests, 257 iMac reference fixtures, complete 42,672-case configured scan, and diagnostic triage.
- Evidence: this record and [original frozen measurements](T069-final-evidence.md).
- Unresolved: original strict payoff misses, four possible wall-control flags, and baseline lexical-thunk gaps below.
- Next action: lead adjudicates the performance evidence; lexical deferred-key context is a conformance follow-up. No promotion, origin push or merge.

## Finding and repair

The lead reproduced a real T069 regression at rebased tip `2e623501`:

```js
class Outer {
  static #k = "m";
  static f() {
    class D { [this.#k]() { return 1; } }
    return new D().m();
  }
}
Outer.f();
```

Main `311bff0e` returns 1; the pre-fix owner runtime throws a private-brand
TypeError. `git diff b93e4daa 311bff0e -- crates` is empty. The rebase did not
change the runtime represented by the original measurements.

Private-name computed keys are deferred into separate bytecode objects.
`resolve_computed_class_keys` reads the enclosing environment's `this` and
passes it to their thunk. The original activation plan looked only at the
containing body's receiver opcodes/slots, so positional entry omitted `this`.
The thunk consequently received the realm's global object instead.

`FrameInitPlan` now combines its own receiver needs with the already prepared
requirements of deferred class keys and lexical arrows. Ordinary functions,
methods, constructors, field initializers and static blocks form receiver
boundaries: their bodies prepare their own nested keys/arrows. They do not
force their parent to materialize a receiver. Computed keys in default
parameters are included because the parameter prologue is part of the
containing bytecode.

The same requirement drives arrow compilation and lexical capture discovery.
This allocates the declaring receiver slot and captures its shared cell even
when the arrow's only receiver read is inside deferred class code. Escaping
and multiply nested arrows therefore retain their creation receiver. Several
such variants also failed on main before T069; this repair fixes them without
introducing a per-call syntax walk or named binding snapshot.

## Coverage and binding audit

Sixteen tests in `function/call_context_tests.rs` cover static/instance methods,
computed method/accessor/field keys, arrows inside keys, escaping nested
arrows, default-parameter classes/arrows, public/static fields and static
blocks (both with and without direct receiver loads), module execution with
and without imports, and the receiver-boundary negative cases. They also
exercise outer `arguments` cells and constructor arrows using `new.target`
and a `super` getter after the constructor returns. The existing 28 activation
tests still pass. A comparison fixture returns `1:2:3:4:11` on both engines.

| Binding or omitted work | Audit result |
| --- | --- |
| Receiver | Fixed: own loads/slots, super, dynamic descendants, deferred keys and lexical arrows contribute to the immutable requirement. Arrow capture compilation consumes that requirement too. |
| Positional parameter names | Inputs still seed indexed slots before capture-cell construction; the existing parameter prologue owns defaults, patterns and TDZ. Rest and mapped-arguments compatibility setup remain. |
| `arguments` | Object admission is unchanged by T069; global names/slots, direct eval and parameter shadowing still control it. Arrows receive the enclosing shared cell. The new test combines an outer arguments read with a deferred-key receiver. |
| `new.target` | Positional entry still runs `insert_super_bindings`; constructors install their target and arrows capture it at creation. The cached own-body opcode flag only affects direct-leaf admission, which already rejects classes and lexical/capturing closures. |
| Home object / super constructor | Still installed from the function or lexical caller by `insert_super_bindings`; T069 does not elide them on positional entry. Constructor-arrow test preserves both target and home object. |
| Active constructor / derived `this` | Existing constructor and lexical-derived branches still install the constructor and shared TDZ cell; completion and field initialization retain the general route. |
| Field-initializer marker / private environment | Existing field/lexical-field branches and `function_private_environment` remain. Private-name lookup and receiver branding are distinct; this fix restores the receiver, not a new private authority. |
| Self / immutable names / realm / module / host state | Existing name-shadow guards, immutable cells, marked-realm setup, module services and host setup remain; foreign/marked realms and imports retain their prior exclusions. |
| Global-this / strict and arguments eval markers | Realm scope and `insert_marked_call_realm` still supply global-this; strict/parameter eval markers are installed at the direct-eval boundary. Arguments markers retain their prior conditions; fresh/inherited call provenance still controls scrubbing. |
| Direct-eval function marker | Positional entry omits it only after `has_dynamic_scope_tree` rejects eval/with anywhere in nested functions, deferred keys, public/private fields, methods and static blocks. Parameter-prologue code is included. |
| Caller name export to native callbacks | Dynamic descendants, retained with/deopt state and marked closure contexts retain their view; static callbacks use shared cells. Deferred-key execution requests `current_env`, not the narrowed native services view. |
| Completed storage | Only cleared local vectors/cold frame storage recycle. Captured cells and suspended/derived ownership are unchanged. |

No additional T069 binding-elision regression was found. Direct-leaf receiver
selection can still use its own-body opcode flag: classes and lexical/capturing
closure creation are excluded before that route is admitted. The broader
positional route uses the repaired transitive requirement.

### Pre-existing deferred-thunk gaps

Separate probes confirmed three context bugs in the thunk execution model on
both freshly built main `311bff0e` and the fixed candidate. They are not caused
by eliding a call-frame binding, and are not encoded as expected wrong results.
QuickJS-NG uses the pinned `f7830186` source in these comparisons.

```js
// arguments: both Rust revisions throw "value is not callable"; NG returns 9.
class Outer { #k="m"; f(v) { return ()=>{ this;
  class D { [(this.#k, arguments[0])]() { return 9; } }
  return new D().method();
}; } } new Outer().f("method")();
```

```js
// new.target: both Rust revisions return "undefined"; NG returns "function".
class Outer { static #k="m"; constructor() {
  this.ok = new (class {
    [(Outer.#k, new.target === Outer ? "ok" : "bad")]() {}
  })();
} } typeof new Outer().ok.ok;
```

```js
// super: both Rust revisions return "undefined"; NG returns "function".
class Parent { static get key() { return "m"; } }
class Outer extends Parent { static #k=0; static f() {
  class D { [(Outer.#k, super.key)]() { return 17; } }
  return typeof new D().m;
} } Outer.f();
```

Deferred keys currently run as receiver-owning function thunks, with their own
arguments/target setup and the new class constructor as home object. Repairing
that lexical thunk model is a separate conformance action. The unchanged
eval-only returned-closure gap remains in [active T069](../T069-call-activation.md).

## Verification and jobs

The repaired runtime is measured against original base `b93e4daa` and the
same five frozen targets. The lead requested one diagnostic triage and no full
new selection/formal/held-out chain unless targets or controls move materially.
No screen attempt is spent or reset, and no promotion command is run.

| iMac queue job | Result |
| --- | --- |
| `20261023T121228Z-t069-context-fix-compare` | Exit 1: new fixture accidentally reused one private name for static and instance fields; invalid fixture, not an engine failure. |
| `20261023T121240Z-t069-context-fix-triage` | Exit 0 at `e38e85fc`: diagnostic unclear; all 105/105 pairs valid, zero invalid/heavy samples, no intruder events; 264.3 s command / 324.9 s including quieting. |
| `20261023T121358Z-t069-context-fix-compare-valid` | Exit 0 at `6c512032`: all 257 reference fixtures pass, including corrected new fixture. |

`6c512032` changes only the comparison fixture; its runtime source is identical
to the measured `e38e85fc`. The corrected fixture was also checked directly on
both local engines before committing it. Every commit passed its installed
change-aware fast gate; the runtime commit ran 2,382 tests and 117 mapped
Test262 cases. The first complete local gate passed 5,170 curated cases;
final full gate and complete exact scan also pass on runtime `6c512032`.

Full gate `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`
passes at `6c512032`: 2,382 runtime tests, 732 benchmark-tool tests, 35 script
tests, all clippy/feature gates and all 5,170 curated Test262 cases. Final
full-gate log SHA-256:
`5666ca1c73a6962f533138ca4b3b5c0601b142b6fec390d7ac3850fa93126112`.
Branch scope validation passes relative to the lead's rebased base `311bff0e`.

### Complete exact conformance

The byte-for-byte unchanged harness scripts ran from worktree scratch space
against copied pinned Test262 `4249661388e5d3f92a85186213da140a6481490f`
`test`/`harness` directories. The actual checkout's `third_party` remained
read-only. The CLI was the release/agents image identified below:

```sh
QJS_AGENTS=1 QJS_CLI_BIN="$PWD/target/release/qjs" \
  TEST262_BASELINE_JOBS=8 TEST262_CASE_TIMEOUT_SECONDS=30 \
  ./target/t069-fix/conformance/scripts/test262-baseline.sh --all --engine both \
  --summary-json "$PWD/target/t069-fix/conformance/summary-both.json" \
  --case-results-jsonl "$PWD/target/t069-fix/conformance/cases-both.jsonl" --no-fail
```

Exit 0; all 53,572 corpus paths appear exactly once and equal the pinned
corpus's complete path set. Configured: 42,672, all Rust pass; zero Rust fail,
timeout, not-run or parity gaps. NG: 42,602 pass, 70 fail, zero timeout.
Skipped by NG configuration: 10,900 (10,621 features, 279 fixtures).
`test262-aggregate.py --require-complete-parity` succeeds on the complete
both-engine case records; a `local-exact-agents` entry for `6c512032` is
appended to `docs/conformance/burndown.jsonl`. No existing record is edited.
All 13 focused script programs also pass through this optimized CLI.

- Complete case JSONL SHA-256: `2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.
- Scan summary SHA-256: `cffdf394963ec66c41d006192593e9975e6b082a55483748550b7a9fb866e80f`.
- New burndown entry SHA-256: `35dbebb46e6c7aa49470ae714e0757e6654a991af52e82734b760e1ad964acda`.
- Complete scan log SHA-256: `7dccb7064a945aafa1febd5724855737d7aab4f329edcfb776a3d12bbe079c2c`.
- Local NG image SHA-256: `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

### Repair triage

Five natural-layout pairs per case, qualified workers-4 collection, no
acceptance claim. No obvious regression under the fixed diagnostic rule;
four possible wall regressions remain: capturing closures, polymorphic calls,
prototype method calls and broad top-level function calls. All instruction
intervals remain within their diagnostic limits.


| Case | Wall C/B [95% exploratory] | Instructions C/B | Previous triage wall |
| --- | ---: | ---: | ---: |
| `development/lodash/transform` | 0.796806 [0.779526, 0.827173] | 0.819394 | 0.795476 |
| `development/immutable/persistent` | 0.927457 [0.888656, 0.949861] | 0.904725 | 0.903438 |
| `development/diff/compare` | 0.980798 [0.946478, 1.054491] | 0.930418 | 0.981168 |
| `development/graphlib/paths` | 0.973078 [0.956679, 0.995199] | 0.976268 | 0.981127 |
| `development/moment/dates` | 1.006777 [0.979018, 1.024093] | 0.986194 | 1.004908 |
| `sentinel/capturing_closure_call` | 1.011779 [0.940952, 1.147002] | 1.000019 | 1.008750 |
| `sentinel/heterogeneous_property_read` | 0.887001 [0.766393, 0.960701] | 0.999830 | 0.974523 |
| `sentinel/polymorphic_call_site` | 1.021896 [0.893129, 1.065657] | 1.000078 | 0.979511 |
| `sentinel/prototype_method_call` | 1.050003 [1.020718, 1.150329] | 0.999247 | 0.935459 |
| `sentinel/recursive_call_tree` | 1.010500 [1.001737, 1.017685] | 1.000006 | 1.004164 |
| `sentinel/string_key_map_churn` | 0.993369 [0.979317, 1.030476] | 0.999960 | 1.010593 |
| `broad/property_dynamic_read` | 0.998563 [0.853952, 1.014033] | 0.995058 | 1.028918 |
| `broad/object_allocation` | 1.018813 [0.994215, 1.037329] | 1.000009 | 0.996519 |
| `broad/string_slice` | 0.972317 [0.931511, 1.077579] | 0.999984 | 0.933597 |
| `broad/top_level_function_call` | 0.986669 [0.969848, 1.408400] | 1.000006 | 0.973678 |
| `external/kraken-1.1/json-stringify-tinderbox` | 1.005278 [0.967638, 1.028306] | 1.001756 | 0.996119 |
| `external/kraken-1.1/json-parse-financial` | 1.006459 [0.967480, 1.010487] | 0.995372 | 0.997234 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.995916 [0.984475, 1.014685] | 0.993189 | 0.991319 |
| `external/sunspider-1.0/access-binary-trees` | 0.989460 [0.981799, 1.014201] | 0.979751 | 0.991083 |
| `external/kraken-1.1/audio-dft` | 1.002333 [0.967745, 1.022421] | 1.000127 | 0.996876 |
| `external/jetstream3-js-subset/cdjs` | 0.966788 [0.955433, 0.990499] | 0.971320 | 0.977930 |

Target wall geometric mean is 0.933816 (previous diagnostic 0.929874);
instructions 0.921391 (previous 0.921258). The largest target instruction-ratio
increase is 0.046%, lodash. This does not establish a material target change.
The prototype-method control moved from 0.935459 to 1.050003 wall C/B; its new
interval [1.020718, 1.150329] and possible-regression flag must not be hidden.
Its point estimate is just below the qualified 1.050127 wall band, and its
instruction ratio is 0.999247 (previous 0.999167). This noisy natural-layout
sample cannot establish layout-independent neutrality or transfer the earlier
formal intervals to the new binary. Given no fixed diagnostic loss trigger,
no full chain was rerun under the lead's instruction. The lead retains the
original strict payoff misses and these control uncertainties for adjudication.

### Identities

- iMac base image: `b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1`.
- iMac repaired image: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- Diagnostic request SHA-256: `2b9c5d0a89ee181cf0237f372b381937ff4bae4205982543d396e6825666956d`.
- Diagnostic result SHA-256: `8ce5d5e78ed2383a80ff4103e7d8e5ea63a811034939e509627f8d47b9c859eb`.
- Successful iMac comparison log SHA-256: `469fcd8ad0e7c2538a0aa4a4574e6ca848bb6b16e42eb9f5e5c4e449eec6a1c7`.
- Local release/agents image: `90bc0d6b52ff55ab89e2bfcc59480ba6331a2ac3d84d7f4a5665bffc8c80e69f`.
- Focused main/fixed/NG comparison JSON SHA-256: `76096e0e6c12c4b4f7db6695b75aeca0f33011b65afeea6def130ca5fe67b0a5`.
- Baseline thunk-gap comparison JSON SHA-256: `8436804df656bd357b15bdcfdade46864a12431b472b2bfb117292c201b330d2`.

Fetched raw jobs live under workspace `outputs/imac-bench/<job>/`; this record
retains the revisions, conclusions, complete diagnostic table and identities.
The original frozen record and plan remain unchanged.

## Handoff

`cargo clean` removed 67,887 files / 7.7 GiB, including scratch corpus/builds;
`git checkout -- target` restored the tracked target fixture. Free disk is
50 GiB. Only handoff records and the new burndown entry are committed after
cleanup, with the fast context gate; no cleaned binary is rebuilt. No origin
push, merge, final performance decision or new dependency was introduced.
