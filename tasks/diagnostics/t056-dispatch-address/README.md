# T056 exact dispatch operands and ops-phase confirmation

[Record](../../archive/T056-dispatch-address.md) owns observations, limitations,
conditional general fix and criteria. Production remains dfff; no owner timing.

`inputs.json` binds count/flags/count_flags and8f/dfff canonical hashes and original
paths. `capture.py` produces unsigned terminal copies preserving headers/sections,
all bytes outside two patch ranges and original caller/frame layout. The result
is current entry addresses, not old timing-process captures. Original60 rows and
probe108 rows are indexed by `captures.json`/`probe-captures.json`, per-role data in
`captures/`. Runtime type blocks prove Vec/frame/cell offsets. No logger/native
call or engine allocation; write syscall occurs after original addresses saved.

`compare.py` follows all152 global/197 local instructions using real bank/ops,
actual jump tables and binary bytes. This is operand-driven replay, not hardware
branch tracing. Each `loops/ROLE-CASE.json` lists every instruction PC/relativePC,
raw bytes,32/64B windows/cross/end flags, ASLR PC and actual next PC; all memory
operands have widths, address class, unslid VM and actual data/process addresses.
RIP targets include both jump tables and+1.0 literal. Summary: `comparison.json`.

Fast count/flags/8f share the PC map; slow count_flags/dfff shift12 instructions
by−3B,24 by−6B,20 by−12B. Same opcode sequence does not mean same PCs/raw code.
Original ops low12 is600 vs900 by class, while local900 is fast; program/bank
modes agree, program+8 vs bank+8 has the same low12 overlap in all originals.
Heap constants/received cell are entry-only. Deferred publication flag istrue,
but sink vector is empty and the wrapper runs once; no hidden per-iteration
poll/publication/ObjectData access. Two coupled factors remain, no unique cause.

## Qualified confirmation copies

- `20261021T124843Z-t056-dispatch-heap-capture`: done0/2s,60 original snapshots.
- `20261021T125859Z-t056-ops-address-qualification`: done0/28s, fourteen exact
  binaries, all global/local N/2N checksums correct. `probe-images.json` records
  every patch and binary hash. Current producer clarifies a docstring only;
  executed payload remains hash-checked and AST-equivalent after that docstring.
- `20261021T130515Z-t056-ops-address-capture`: done0/4s,108 probe snapshots.
  Forced600/900 always correct. flags-sham has two original-allocation outliers
  (900/180) retained; sham is never treated as a forced phase.

`probe_build.py` modifies COPIES only. Each entry maps one RW4K page and copies
ops to600 and900, then chooses the original/sham or a copied pointer. Exact
same setup is used for both phases. Exit restores original Vec pointer and
RSP before original cleanup; page is bounded until process exit. Entry tail
and exit are redirected; temporary RSP reserve is32B, RBP/old spill addresses
remain fixed. Repeated152 bytes/PCs/RIP targets are all unchanged. No signature,
header, function extent, heap allocation before entry or per-iteration code edit.
Only the qualified numeric global/local programs may run these diagnostic copies.

## Lead-only single job (≤15min)

```sh
python3 tasks/diagnostics/t056-dispatch-address/render.py > /tmp/t056-ops-confirmation.sh
/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run \
  --repo "$PWD" --ref HEAD \
  --with f58c696e8379261ed726f0cea1051ed6d34f431a \
  --with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5 \
  --name t056-ops-confirmation --timing \
  -- bash -c "$(cat /tmp/t056-ops-confirmation.sh)"
python3 tasks/diagnostics/t056-dispatch-address/watch.py JOB_ID
```

No owner submission. Exact policy/unit, correctness/86-case receipts, host,
probe hashes and actual snapshot phase qualifications are checked before costs.
Fourteen images vsf58, two sham A/A clones, global/local, N/2N, five shared
paired blocks:17roles/170units/340 measured+170preflights. One active pathname,
independent inodes, rotate/reverse order, clean legacy env. Instructions/wall;
no untrusted cycles or CPI. Qualification snapshots are not measured-child
addresses; forced phases follow the verified code, sham original phase can vary.

Sham/original AND same-phase-relocation/sham must be neutral (median/95%CI within
[1/1.05,1.05], instruction median±1%) on both cases. Sham A/A±3%, CI±10%,
instruction±1%; anchors count/flags≤1.05,count_flags/dfff≥1.30 vsf58. Then slow
600 restoration plus fast900 reproduction supports phase causality. One-sided
change is source-dependent; neutral/no effect rejects sampled phase explanation;
failed controls remain inconclusive. All rows kept, no address-based cost filter,
retry, best phase, architectural fix or acceptance decision from this job.

Real previous throughput gives≈145.3s;3x+120s<720s. collect720/body840/runner890s
and watcher900s including quieting, queue wait excluded. Host local03–05 plus
15min pre03 margin excluded; admission requires20min. Failures/timeout/disturbance
preserve partial output and stop; watchdog targets own descendants only.

## Recheck

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-dispatch-address/validate.py
```

Requires fetched named jobs; `T056_DIAGNOSTIC_QUEUE` overrides their parent.
Validates exact binary patches, repeat bytes/PCs/windows, real60+108 snapshots,
retained outliers and actual replays. Timed collector runs over fake processes
only:510 calls, result routing, guards, output/hash/timeout/disturbance failures,
15min/DST/window and rendered syntax. `validation.json` is not timing evidence.
