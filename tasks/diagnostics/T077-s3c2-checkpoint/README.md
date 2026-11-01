# T077 S3c v2 C2 checkpoint

Measurement base: `8322fc983da20800ce612ad148781aea30e28d08`. Branch:
`agent/t077-s3c-v2/codex`, based on main `e60e7cfa`. Runtime source is the C2
commit containing this receipt; `builds.json` seals its exact Rust/Cargo inputs
and four candidate binaries. Pre-commit request HEADs name parent `a8ae898c`;
their input hashes match this C2 source. No old-worktree files were modified.

**The revised checkpoint is missed. Stop; C3–C5 are not started.** The debug
resource gate and two of three call-instruction targets fail. This is neither
S3c acceptance nor permission to change any target. The lead did not authorize
combined S3c+S3d. No runtime changes follow the failed checkpoint.

## Implementation and checks

`bytecode/frame_bank.rs` owns contiguous value/header stacks. Inline calls push
64-byte headers, move arguments directly and clear compiler-declared written
ranges plus used prefix slots. Both production targets compile the size/offset
assertions. No per-inline-call Box, directory traffic, FrameId or code copy is
introduced. Current APIs retain direct owners, so no directory ID is needed.
RuntimeServices retains only cleared stack capacity (64 / 256 KiB / 16 KiB,
including conservative free-entry bookkeeping), never semantic Context owners.

`context/activation_borrow.rs` borrows prefix inputs and immutable creation facts.
Context materializes on real service demand; refs then select it as authority.
Empty-reference-bank/module metadata queries do not materialize Context. The
3/3000-call counter test covers both captured-cell and global writes. The first
draft accidentally materialized Context to check `saved_references.is_none()`:
4 constructions at 3 calls, 3001 at 3000. That bug was repaired before freezing
this candidate. Captured-cell Assign and reference selection policy are unchanged.

ActivationStack, both TLS owners, the services activation pool, eligibility,
rebind and release helpers are deleted. Old reuse/poison tests now prove storage
clearing, escaped-owner separation and corruption detection. C1 `a8ae898c`
retains one box through suspendable execution and uses unboxed ordinary VM
records; cached root/eval code is borrowed without copying, and TLA retains Rc.

Frozen-source correctness passes:

- full `check.sh`: 2,733 runtime tests, feature/tooling checks, 5,174 curated
  Test262; full perf-counters: 2,784; x86_64 compilation/layout checks;
- seven stress/poison configurations: 2,733 each; combined IC-off: 2,691 with
  the unchanged 42 cache-mechanism exclusions;
- NG 322 scripts and 10 modules in each of nine modes, using the documented
  optimized assertion/overflow-enabled verifier and ordinary 10-second budget;
- public sweep: **351/351**, no failures, original deadlines;
- fuzz: **500 seeds × four profiles**, MATCH 1,791 / pre-existing differences
  209; stable inputs, no new regression or mode disagreement.

`checks.json`, `stress.json`, `compare.json`, `public/` and the fuzz records own
these results. Full exact Test262, formal/held-out/RSS acceptance remain C5 work.

## Local instruction targets

`call_cost.py` uses the median of three retired-instruction samples at N=1M and
2N: `(I(2N)-I(N))/N`. `call-cost.json` retains source hashes, samples and reach
counters. No local timing/cycles are retained.

| Case | S3b instructions/call | C2 | Saved | Required saved | Met |
| --- | ---: | ---: | ---: | ---: | --- |
| upw0 | 1003.053508 | 931.959091 | 71.094417 | >=70 | yes |
| upw1 | 1333.111299 | 1267.121607 | 65.989692 | >=70 | no |
| upw4 | 2275.179170 | 2242.183630 | 32.995540 | >=70 | no |

Every N=1M case retains 1,000,002 ordinary attempts, 1,000,000 compact direct
calls, 30 general ops and 105 cell allocations. Before/after compact ops are
identical: 9,000,009 / 13,000,009 / 25,000,009 respectively. C2 materializes one
Context per process, not per call. This rules out a tier change, extra semantic
work, eager per-call Context or per-call cell allocation as the cause of the
shortfall. The 45+3a+3w lifecycle estimate has not been isolated in the generated
driver; its expected net payoff is not demonstrated on the captured-write bodies.
The remaining shortfall is implementation/dispatch cost at unchanged semantic
reach, rather than a newly selected language path. No ablation
isolates those sub-costs here; do not assign unsupported per-piece numbers.

`controls.py` seals five interleaved samples per public control, validates stdout
and retains both event maps. `controls.json` contains all samples. The supplied
method-call reproducer and the broad method control are both reported to remove
any ambiguity in the feedback's name.

| Control | Instructions C2/S3b | <=1.00 |
| --- | ---: | --- |
| access-fannkuch | 1.000067523 | no |
| hash-map | 1.022033145 | no |
| date-format-tofte | 0.979786005 | yes |
| supplied method-call.js | 0.981650158 | yes |
| broad/method_call | 1.002684062 | no |

Fannkuch's old 5.3x sticky-decline loss is absent; its tiny remaining excess is
still a strict target miss, not rounded into a pass. Tofte uses the original
clock-sensitive public input; its fixed five-sample result is reported intact.

## Depth failure attribution

`depth.json` retains every boundary probe for both frozen binaries, with the
ordinary CLI main thread and no stack-size override. Plain is censored at 4096.

| Case | Debug S3b | Debug C2 | Release S3b | Release C2 |
| --- | ---: | ---: | ---: | ---: |
| forEach | 183 | 163 | 935 | 951 |
| map | 182 | 162 | 927 | 942 |
| reduce | 236 | 206 | 1026 | 1073 |
| call | 437 | 352 | 3448 | 3915 |
| apply | 366 | 304 | 2335 | 2528 |
| Reflect.apply | 367 | 305 | 2491 | 2712 |
| yield* | 152 | 204 | 820 | 1251 |
| plain in callback | 4096 | 4096 | 4096 | 4096 |

The debug disassembly directly attributes native-stack growth to the rewritten
unoptimized drivers. `debug-stack-attribution.json` preserves exact prologues:
wide S3b `run` reserves 15,408 bytes; C2 `run_root` reserves **19,952**, +4,544.
Numeric `run_frames` grows 5,504→7,664, +2,160. Call entry grows only 2,384→2,400;
`call_with_arguments` grows 3,376→3,392. The driver temporaries/native frame,
rather than boxed-per-call storage or eager Context, are the demonstrated depth
problem. Release depth improves in every measured ceiling. No CLI/resource
policy change hides the debug loss, and no repair was applied after this gate.

## Reproduction and stop

See the versioned collector scripts here. Build default debug/release and a
separate release perf-counters CLI. The verifier recipe is:
`cargo build --release -p qjs-cli --config profile.release.debug-assertions=true
--config profile.release.overflow-checks=true`, in a separate target directory.
Run `verify.py stress`, `verify.py compare`, the documented public sweep and
`tools.binding_fuzz.run --count 500` for all four default profiles. Source/binary
seals identify each build; poison/counters are never timed.

The single requested iMac `t077-s3c2-c2` public86 run is recorded below when
complete. Its collection completes the requested evidence, not a runtime retry.
Lead review is needed before any debug-frame or remaining lifecycle/codegen
repair. C3–C5 are blocked by the missed revised checkpoint.
