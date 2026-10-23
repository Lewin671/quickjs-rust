# T071: Lexical context in deferred keys and eval closures

- Status: closed-landed on the owner branch only; not pushed to origin or merged.
- Implementation: present in `d7fef572` and final runtime `49d14c01`.
- Verified at: `49d14c01`, 2026-10-23; fast/full local gates, 259 reference fixtures, and 9,226 configured Test262 cases in five complete area scans.
- Evidence: this record; focused tests and comparison fixtures below; remote jobs and log identities below.
- Unresolved: none in the four requested defects; independent general eval const-write gap below.
- Next action: lead can review/integrate separately; the next independent correctness item is eval const-write enforcement. No push or merge is authorized here.

## Goal and architecture

Repair the four baseline programs recorded in [T069 thunk gaps](T069-call-context-fix.md)
and [T069 eval gap](T069-call-activation.md). Deferred private-name class keys
must inherit their enclosing lexical function context. Eval-only returned
closures must retain the enclosing bindings their eval source can name.

The target uses existing indexed slots and shared upvalue cells, never a
per-call name-keyed snapshot. Stages were: reproduce, repair compilation/cold
class evaluation, verify focused cases and complete area slices, run gates,
and record evidence. No dependency, unsafe code or third-party source change.

## Scope and final design

Base `6e899d24c167d1393c077bcd69a01b61571dbcd1`; branch
`agent/t071-lexical-context-gaps/codex`; worktree `t071-lexical-context`.

- `bytecode/compiler_class.rs` and `vm_class.rs`: deferred keys were ordinary
  receiver-owning thunks with their own arguments/target and the new class as
  home object. They now use lexical this/arguments, retain target/home/super
  context and derived-this cell identity, and explicitly install the new
  class's private environment. Escaping arrows inherit those same authorities.
- `compiler_lexical.rs`: identifier-only discovery missed names occurring only
  in eval strings. Dynamic descendants now force compile-time capture of
  visible outer bindings and forward cells through intermediate closures.
  Realm vars retain name lookup; compiler temporaries never enter captures.
- `compiler.rs`, `compiler_expr.rs`, `compiler_values.rs`: function-scope
  ownership follows hoisting and direct lexical declarations, rather than
  treating every nested block/catch spelling as an own function binding.
  A block-local name therefore cannot suppress an outer capture visible outside
  that block. Class compilation consumes the same ownership analysis.
- `compiler_try.rs`: implicit function slots discovered inside a try survive
  its exception cleanup. The exemption checks the actual function-owned slot,
  so a source `let arguments` still clears. This preserves both derived-this
  after a caught TDZ read and the enclosing arguments binding after a throw.
- Tests: `function/lexical_key_tests.rs` (8),
  `tests/direct_eval/outer_bindings.rs` (9), and the corrected known-divergence
  assertion in `tests/global.rs`. Compare fixtures are
  `class-key-lexical-context.js` and `direct-eval-outer-cells.js`.
- Current binding facts are in `docs/design/env-model-rewrite.md` and module
  comments. No ordinary call/frame executor or dispatch hot-path code changed;
  runtime edits are confined to deferred-key creation. Dynamic closures retain
  cells required by semantics. No timing triage was triggered or acceptance
  chain run, and no performance neutrality claim is made.

## Focused findings

On the base, 6 of the initial 7 valid key/context tests and all 6 initial eval
regressions fail. The escaping private-environment case already passes and
protects the separation of private authority from home object.

Review of `d7fef572` found its initial cleanup exemption also retained a source
`let arguments`: `function f(v) { try { let arguments='bad'; throw 0; }
catch(e) {} return eval('arguments[0]'); } f('ok');` returned `b`.
`49d14c01` restricts the exemption by slot ownership. The regression test also
creates a deferred key in the try, proving implicit arguments survive while
its explicit lexical shadow clears. This is included in the final fixture.

## Acceptance and local gates

- [x] Four reported programs and lexical-context variants match QuickJS-NG.
- [x] Shared writes, transitive captures, block/iteration ownership, TDZ,
  const reads, own names/parameters, defaults, methods and indirect-eval boundaries.
- [x] Complete baseline/final scans of all five selected Test262 areas.
- [x] Fast/full gates, fixtures, branch scope, local commit and cleanup.

At final runtime `49d14c01`:

- `check-touched.sh --staged --explain`: exit 0; 2,399 runtime tests and
  117 focused Test262 cases (eval-code, block/function statements, arguments,
  call/function fixtures, function-code and call expressions).
- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`:
  exit 0; format; all three clippy configurations; agents/perf-counters tests;
  workspace tests (2,399 runtime); 732 benchmark-tool tests; frozen-plan
  validations; context; 35 script tests; hook isolation; shell syntax; file-size
  guard; 5,170 curated Test262 cases.
- Branch descent/path scope against `6e899d24`: exit 0.
- Full log SHA-256: `21d2a39d549d21ed5d8ed17ce6f407ba329219199bd8ed5ebfffdea768791708`.
- Fast log SHA-256: `ffe3b7545f409c01716368e29c61f40a9bd3c2098c8fa929aa881cfe2633f784`.

Raw local logs are not retained; this record retains results and revisions.
The preliminary gate stopped at document budgets (index 121/120, binding note
167/160); both were condensed in place. A refinement draft's nonminimal-bool
clippy diagnostic was corrected before the final passing gates.

## Complete area comparisons

Each command was `find-qjsng-gaps.sh --filter <area> --all`, scanning the exact
requested range. These are filtered scans, not a full-suite burndown record.

| Area under `test/` | Base Rust pass | Final Rust pass | Feature skips |
| --- | ---: | ---: | ---: |
| `language/expressions/class` | 4,049 | 4,049 | 10 |
| `language/statements/class` | 4,351 | 4,351 | 16 |
| `built-ins/eval` | 10 | 10 | 0 |
| `language/eval-code` | 347 | 347 | 0 |
| `annexB/language/eval-code` | 469 | 469 | 0 |
| Total configured | 9,226 | 9,226 | 26 |

All configured Rust cases ran: zero failures, timeouts or NG-pass/Rust-nonpass
gaps, both before and after. NG alone fails the two class-statement cases
`elements/syntax/valid/grammar-field-named-{get,set}-followed-by-generator-asi.js`
on both revisions; Rust passes them. No new reference-side discrepancy.

## Reference jobs and identities

All jobs used the iMac queue without timing; refs were fixed commits.

| Job | Revision | Result |
| --- | --- | --- |
| `20261023T125304Z-t071-before` | `6e899d24` | Exit 0, 312 s; class expressions/statements and built-in eval baseline. |
| `20261023T133442Z-t071-eval-before` | `6e899d24` | Exit 0, 35 s; language and Annex B eval baseline. |
| `20261023T130607Z-t071-compare` | `d7fef572` | Exit 0, 12 s; 259 fixtures (intermediate runtime). |
| `20261023T130608Z-t071-after` | `d7fef572` | Exit 0; same 8,410 class/built-in eval passes (intermediate runtime). |
| `20261023T133443Z-t071-eval-after` | `d7fef572` | Exit 0, 36 s; same 816 eval-scope passes (intermediate runtime). |
| `20261023T134323Z-t071-final-reference` | `49d14c01` | Exit 0, 350 s; all 259 fixtures and all five complete area comparisons. |

Final fixture outputs on both engines:

- Class keys: `9:function:true:17:18:23:a:4:true:true`.
- Eval cells: `9:10:10:9:10:11:0,1,2:true:4:6:3:8:8:8:ok`.

Log SHA-256 identities, in the job order above:

- `4c0190aa2bc48db4fc89e5307ae8b58d42b16ea385d719d2b08233492ab3fd38`
- `82739afb25e0cb9cc1e3501fee6a1640ab4261c56c6ce80b7e4349d86801e7d8`
- `ebf2cc9fec8321de028215add9a9a2109c19ba8342822656a19e4b2a722013da`
- `a73c5e462a47ef5be7680535874f49290542990eca78a0544898976f59350842`
- `4b55b5ae9b199a35a3627cc5a7c14d82669598d75f536a0ae17f68e78f3d2bf3`
- `8f935e02bafbe6bce77a813efe2a269ea4095933ea2154cb9ab6b6e47a45a0e9`

Fetched artifacts are in workspace `outputs/imac-bench/<job>/`; this record
retains the conclusions, numbers and revisions independently of those files.
Remote CI was not run because no origin branch publication was requested.

## Independent follow-up

Ordinary direct eval allows a write to a function-local const:
`function f() { const fixed = 3; eval('fixed = 5'); return fixed; } f();`
returns 5 instead of throwing TypeError. This program has no nested closure
or deferred key; local const mutability in the general eval environment is a
separate task. Captured-slot mutability and const reads/TDZ remain covered here.

## Cleanup

`cargo clean` removed 11,754 files / 7.5 GiB; `git checkout -- target`
restored the tracked fixture (target directory 4 KiB). The final evidence
commit changes only task documents; its fast gate checks context without
rebuilding binaries. No origin push, merge or new dependency was introduced.
