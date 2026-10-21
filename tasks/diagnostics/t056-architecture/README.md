# T056 architecture diagnostics

[Chinese review](../../archive/T056-architecture-review.md) owns conclusions,
limits and the migration proposal. No production implementation is added here.
All completed jobs below used the serialized iMac runner **without --timing**;
their elapsed times are not performance measurements. The data-first timing probe below is retained as historical preparation.
The current lead sequence is the [code-layout mini-panel, then code×zone](../t056-code-layout/README.md);
none of these timing probes has been submitted by this owner.

## Completed non-timing evidence

| Job | Result |
| --- | --- |
| `20261021T071030Z-t056-architecture-address-capture` | failed: missing PYTHONPATH; no address evidence |
| `20261021T071414Z-t056-architecture-capture-v2` | failed compile: unqualified diagnostic Rc; no address evidence |
| `20261021T072833Z-t056-architecture-capture-v3` | done0,214s; base/8f/dfff, two processes each, global N/2N, local1m; unpack initially excluded by loop-only filter |
| `20261021T071529Z-t056-architecture-instruction-attribution` | done0,257s; full/header/adapter/write-index ablations, three counter pairs per four cases; instructions only |
| `20261021T075152Z-t056-architecture-pad-build` | done0,143s; two padded diagnostic images, three factors×NG255, all four modes×five workload outputs match |
| `20261021T075245Z-t056-architecture-supplement` | done0,218s; all wide returns, three versions; global/local seed-cell identities and 92,000 unpack callback returns/version |

`instrument.py` + `build_capture.py` reproduce v3. `instrument-supplement.py`
+ `build_supplement.py` reproduce the supplemental capture. Both patch only
fresh git archives outside the checkout. Runtime fields and allocator API are
unchanged; logging after execution can change code/stack layout and later
allocation history. These are **diagnostic-image pointers**, not untouched
canonical-image pointers. Supplemental wide pointers describe returned backing
buffers, not every transient native operand during the call.

`address-evidence.json` preserves all v3 pointer records, selected canonical
numeric-path byte audits, raw-file hashes and ablation instruction medians.
`capture-identity.json` binds source revisions/recipe/images. The supplement
record retains every typed pointer/seed plus the first return per each of eight
wide programs and all return counts; `supplement-identity.json` binds its raw
files/images. Complete compressed logs, archived source, binaries and full
ablation results remain in the named runner job outputs; conclusions and
representative actual addresses are also committed in the review.

`ablate.py` changes source archives only. Counter totals include downstream LLVM
changes, not just the cost of edited source lines. Static graph facts in the
review are from canonical disassembly; deleted standalone symbols can mean
inlining, not missing execution. Canonical numeric audits use this directory’s `audit.py` and
`extract-paths.py` (the earlier data-probe audit adapted for these two hashes); the committed audit records
retain raw bytes, decoded instructions, and checked address relocations.

To render a reproducible **non-timing** payload (does not submit it):

```sh
python3 tasks/diagnostics/t056-architecture/render_build.py supplement > /tmp/t056-architecture-nontiming.sh
# Other modes: capture, ablation, padding.
```

Submit only through `imac-bench.sh run --repo "$PWD" --ref <revision>` with
`--with f58c696e8379261ed726f0cea1051ed6d34f431a`,
`--with 8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a`, and
`--with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5` available.
Do not add --timing for these producers. The archive builders use
`tools.benchmark.build_cache` and its standard x86_64 fat16/abort/agents recipe.
The ablation expects the two original canonical images already in that cache.

For an existing canonical binary pair, the read-only byte audit is:

```sh
PYTHONPATH="$PWD" python3 tasks/diagnostics/t056-architecture/audit.py --base /path/to/f58/qjs --candidate /path/to/dfff/qjs --output-dir /tmp/t056-architecture-byte-audit
```

The candidate is restricted to the two reviewed canonical hashes; the base hash
is fixed. `hot-type-sizes.json` retains relevant sizes/alignments from the
previous base/8f compiler inventory; the corresponding type definitions are
unchanged by dfff. `validation.json` records the local fake-process verification.

## Historical data-first robustness probe (not the next lead job)

**Question:** does changing unrelated live allocation sizes change the wall
cost of the same interpreter image, and does that effect differ between8f and
dfff? This diagnoses current baselines. A future arena prototype needs its own
prospective config and qualifications; do not replace a sealed image here.
It is not an acceptance test or a final K=6 attempt.

Padding images alter **only CLI main**, before JS startup: hold64 touched byte
buffers of128/144/1536 bytes until after execution. `QJS_ARCH_PAD_MODE` values
`pad0128`, `pad0144`, `pad1536`, `pada128` all have length7; last is128 A/A.
The holder capacity and runtime source are unchanged within an image. Metadata
and full image hashes are in `pad-build-identity.json`: aligned image
`c80cebb5fec9bf21cc3980bb75cb73f58e7175b2ad5ff28200d5f14b28fb7f22`, allocation
image `d237ca571d51bd20592ea5439bf07afc4ccd51800111c7227f1c98db12565f2d`.
The build checked six NG255 runs and eight matching-output groups; neither
these images nor qualification process durations are canonical performance.

Six cases: global_read, local_read, branch_arithmetic, heterogeneous_property_read,
whole unpack, and empty. Every case has12 roles: each version's canonical,
canonical A/A clone, pad128, pad144, pad1536, pad128 A/A clone. The two A/A
clones have identical bytes but separate inodes; three real pad factors share
one inode. Every launch uses one active pathname. Each role resets allocator,
old trace/cookie keys; TZ=UTC/LC_ALL=C. No machine settings/services are changed.

15 blocks rotate/reverse case and role orders; alternate N/2N order. Internal
fixed N:23,068,672(global/local),13,182,098(branch),2,883,584(heterogeneous).
No calibration, factor selection, block deletion or retry. Retain raw process
outputs/counters, source/image hashes, inodes, effective overrides, all samples,
per-case paired effects and95% block-bootstrap intervals for instructions/wall.
Report pad144/128, pad1536/128, both A/A, C/B and interactions; canonical-vs-pad
is a **bias control** because CLI changes also change codegen/setup. Whole unpack
includes buffer allocation/free cost; report empty alongside, never silently
subtract it. Internal difference cancels constant work, not possible N/2N
address differences. No diagnostic pass/fail or promotion rule is manufactured.

Address perturbation is an experimental validity condition: compare the
non-timing captured active streams under the fixed pad factors before claiming
a mechanism, using a separately labelled instrumented mirror if needed. That
mirror's pointers do not prove canonical pointers. If relevant addresses do
not change, old code has no detectable sensitivity, or AA is broad, a stable
candidate would be inconclusive evidence of a cure. This first job measures
sensitivity; it cannot itself establish that an unimplemented arena fixes it.

Budget: previous540-unit probe took401s; this1080-unit matrix projects802s,
×1.4 plus120s reserve =1243s. Collection hard cap1260s; body cap1380s;
whole running job including runner quieting cap1500s. Projection is not a
completion guarantee: an over-budget job fails, retains every partial pair,
and is not used for a verdict. The body rejects insufficient remaining budget;
lead watchdog stops only this job's process tree. Queue wait is excluded.
Host-local03:00–05:00 is excluded, next03:00 has15-minute margin, admission
needs30-minute headroom. Host timezone comes from /etc/localtime, not inherited TZ.

The collector verifies all prerequisite done0/no-DISTURBED receipts: original
dfff correctness, bound86, diagnostic likely-loss triage; padded build metadata,
canonical image hashes; frozen unit hash and unchanged production source.
`chain_common.py` is byte-identical to the existing dfff receipt helper
(SHA `24f2e155...`), so old-candidate/failed receipts cannot satisfy it.
It rejects a non-timing runner invocation. Config seals collector/protocol hashes.
`validate.py` exercises actual collection with **fake processes/counters only**,
real sealed metadata, environment resets, A/A inode identities and all1080 units;
it submits nothing and starts no engine executable.

After independent design review, **lead may submit**, from the committed branch:

```sh
python3 tasks/diagnostics/t056-architecture/validate.py
python3 tasks/diagnostics/t056-architecture/render.py > /tmp/t056-architecture-robustness.sh
# LEAD ONLY: do not execute this as part of owner handoff.
job=$(/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run --repo "$PWD" --timing --ref HEAD --with 8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a --with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5 --name t056-architecture-robustness -- bash -c "$(cat /tmp/t056-architecture-robustness.sh)")
python3 tasks/diagnostics/t056-architecture/watch.py "$job"
```

The wrapper and watchdog are both required. Fetch the result even on timeout;
only `complete.json` plus all15×6×12 samples constitutes a complete diagnostic.
Old K=6/v6 plans, policy bands and remaining attempt count stay unchanged.
