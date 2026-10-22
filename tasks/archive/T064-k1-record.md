# T064 K1: production typed service boundary

- Status: blocked after repair 1; all 86 pre-gate cases complete, the same 46 still fail. Implementation retained on `agent/t064-k1/codex`; no timing or measured stage decision.
- Implementation: 20 non-inlined object/boxed arms shared by both frame roles; repair `773c42f7` removes only the service cold hint. Exit paths remain cold/non-inlined. No crate, dependency, unsafe, FFI or recipe change.
- Verified at: repair `773c42f7` full/fast gates, 256 NG fixtures, 5,170 subset; three-image/four-case attribution complete; full static matrix PASS for both roles with identical local replay; repaired instruction pre-gate BLOCK (46/86). Original `671e9943` evidence retained below.
- Evidence: this record; [instruction pre-gate](../diagnostics/t064-k1/pregate.sh); [contract](T064-kernel-isolation.md#lead-decision-2026-10-21).
- Unresolved: instruction growth across 46 policy cases still blocks screening. Dominant sampled owners are split typed hot/service execution; exact retired-instruction deltas per symbol remain unknown.
- Next action: return the completed one-round attribution and failure evidence to the lead. Stop; do not repair again or screen this candidate. Thresholds and timing-attempt ledger are unchanged.

Imported from `agent/t064-k1/codex` evidence commit `06682ecc`; runtime
commits `671e9943` (K1) and `773c42f7` (Repair 1) stay on that branch and
are not merged by this documentation import. This owner record precedes the
[lead closure](T064-kernel-layout-authority-closure.md) on 2026-10-22;
its stop/report next action was fulfilled by that decision.

## Boundary and semantic coverage

The hot function is non-inlined. One cold non-inlined service executes each
non-scalar arm completely; its false result invokes the caller's unchanged
current-site deoptimization. Writeback, deopt and stack materialization stay
cold and non-inlined. Success does not change the PC or residency accounting.
Register banks, receiver roots, caches and intrinsic state are borrowed in
place. There is no projection buffer, additional transported bank, new
allocation, admission change or new binding authority at the service boundary.
Original allocations performed by operations such as ArrayPush are preserved.

The 17 fallible arms each have a forced rejection test after a scalar update
and an array write. It checks exact PC 73, mixed stack order, both slot classes,
unchanged destination registers and the completed write. The three infallible
arms (MoveBoxed, Truthy, Box) have a normal-exit test. Observer tests exercise
Proxy get, accessor get/set, callback shared cells, direct eval, with, TypedArray
buffer detachment and foreign-realm data/intrinsic guards. JS observer cases
run both at script scope and in a function; direct forced-exit tests use Vm.

Feature-gated counters record dispatch entries, service calls/failures,
slot/stack publications, fresh scratch banks, growable-buffer growth events,
and scratch live/retained byte high-water. Growth events are not a claim about
all allocator calls. No instrumentation exists in production builds. No
projection or bank-copy bytes are introduced (zero by construction); slot and
stack publication counts describe the existing exit protocol. Scratch pooling
still retains at most one cleared bundle per program. Numeric dispatch tests
re-enter the same compiled program twice and require zero services/growth,
one scratch allocation and no growable scratch payload.

## Verification receipts

Candidate `671e9943` is based on `d1e80fec`; its runtime diff against fixed
M `9eba7a1d` contains only K1's execute/service/tests, typed scratch counter
sites and diagnostic counter definitions (six files). No unrelated runtime
change, recipe edit or new dependency. `execute.rs` shrinks 1,793→1,516 lines;
service is 370 lines, forced-exit tests 266 and observer tests 102.
Review against M confirms all 20 moved arm bodies retain the same ordered
operations after replacing deopt/continue with false/true returns; scalar
arms, admission, residency and shared-slot/cell protocols are unchanged.

Local full `scripts/check.sh` exits 0 at `671e9943`: 2,342 default runtime
tests, 24 diagnostics tests, agents tests, both feature clippy passes,
723 benchmark-tool tests, 35 script tests, context/file-size/hook checks and
5,170/5,170 curated Test262 cases. Focused agents+perf-counters execute suite
passes 13 tests, including the parameterized 17 fallible arms. Staged fast
gate passes, including 65 selected semantic Test262 cases. NG comparison
passes all 256 fixtures. The checkout's pinned submodules were initialized
without recursive/nested initialization; no third-party source changed.
The first NG invocation stopped on the uninitialized reference build; the
first full gate stopped on a test initializer lint, and a later full gate on
a four-line documentation-budget excess. Both were corrected before the
final complete passing gate; no failing check was waived.

| Local log (not retained) | SHA-256 |
| --- | --- |
| Full gate | `79304acab595cdf872b2ce4acf185cbfdcc6d38f2dd20704fb2efbd6ee2e9887` |
| NG comparison | `9f2dc6f7fe1468d6f4175c1e9828d6823c1b2ce0e515d1ab8334b868917bbb5f` |
| Fast gate | `51e77f2e3492a0693665fb625a64b0f5e4e88253d288ebbc967124ea0e42365d` |

Queued jobs (both without `--timing`):
`20261022T064201Z-t064-k1-final-static` (one `--full` matrix) and
`20261022T064347Z-t064-k1-counts-pregate` (agents/perf-counters typed tests,
existing WideLoopFrame diagnostics, then exact-image 86-case instruction
pre-gate). The static job exits 0; the counts/pre-gate job exits 3 for the instruction block after 298 seconds of diagnostic work. The K failure budget remains unused;
old `0a9af81b` and its shared A+B reserve are unchanged. No timing screen,
K=6 panel, formal comparison, branch push or merge was run.

## Static matrix: committed candidate

Job `20261022T064201Z-t064-k1-final-static` exits 0 after 421 seconds of
build/check work (not a timing result), producer/source `671e9943`. Exactly
one `--full` run. Rust1.95.0/LLVM22.1.2, x86_64-apple-darwin, canonical
fat16/abort/agents recipe, unchanged checker implementing the lead contract.
A/A whole images are identical. The separate layout witnesses are effective:
R0 112/400 B, H+8 120/400 B, C+8 112/408 B (header/cold). H shifts brand
fields 108/110→116/118; C retains header offsets. Source line preservation
and probe-free canonical images are recorded in builds.json.

| Comparison | Vm | WideLoopFrame |
| --- | --- | --- |
| A/A | PASS, 0 failures/allowed fixups | PASS, 0 failures/allowed fixups |
| H+8 | PASS, 0 failures, 82 allowed fixups | PASS, 0 failures, 83 allowed fixups |
| C+8 | PASS, 0 failures, 25 allowed fixups | PASS, 0 failures, 25 allowed fixups |

All H/C allowed entries include an instruction displacement; nine per role
also have verified referent-pointer relocation, not nine extra entries.
Extent/instruction/table counts stay Vm 6,320 B/1,346/3 and Wide
6,272 B/1,331/3. Stack reserves stay 0xd8/0xc8. Both roles retain exactly
one service call site (Vm +0x1409, Wide +0x13ff); numeric operations do not
traverse that site. Non-address bytes, instruction boundaries/relative PCs,
owned table bytes/targets/positions and stack reports all match.

| Image/receipt | SHA-256 |
| --- | --- |
| r0a/r0b | `0847b950b2279dbf194cca7bdf234e87b1b8410ea38f6177e64fa458e3b67a30` |
| h8 | `5e983ff71ed9701023b4fe48e8ff116393ba3d12c201a158b123b1ffadc57a27` |
| c8 | `39c72ef7cd18dbec985028316a3b87aa07551753b075b6784fb39da6714e3899` |
| request.json | `217f4ad5614236f74377aecab4e76422e0dcb6c2ad28d360d4ea3fa898c7c901` |
| builds.json | `ca89d8b90f4d140be3b1c9d5b580a360f09380bd7d8a7d03a26d082b6b57bce0` |
| comparisons.json | `541982ce5f8877bb2bcd2efc817085715f41cfef6b0d8aed6f1b23094dffb7e1` |
| result.json | `1c8a0b518afe1436a8966576a6395dfbd7faacd6b52458c94f19d56076382257` |

After fetching, local unmodified checker code verified every image/source
SHA and re-fingerprinted all four images. comparisons.json is byte-identical
to the queue result despite the independently qualified local otool decoder.
This finite static result is neither a timing result nor proof for other
perturbations, targets, recipes or toolchains.

## Work/service counters

The second job passes 111 typed-loop tests and all 24 existing execution
counter diagnostics, including the wide-tier deoptimization/receiver/stack
cases, with agents+perf-counters enabled. Numeric test (two entries into the
same compiled scalar program): 0 services, 0 service deopts, 1 initial scalar
scratch allocation, 0 scratch buffer growths, retained capacity 4,096 B,
live growable payload 0 B, 4 slot and 2 stack publications at exits. There are
no per-iteration service calls or scratch allocations, and no projected bank
or additional copied bytes. The retained bank is reused on the second entry.

Each of the 17 forced service-exit cases reports 1 dispatch entry, 2 service
attempts (the completed array write and the declined arm), 1 service deopt,
2 slot publications and 2 stack publications. Scratch is preconstructed in
these direct-execute tests, so their zero scratch counters do not measure
wrapper allocation. Allocations made by original user-visible effects are
not claimed to be covered by the scratch counters.

Counter log SHA-256 `3bd82f0ac3f866c7ba17c21a6fb2ea18b2c257c72522e21619fd1680f87be4c6`;
existing diagnostics log `1f44c5eef2a74a00d0095ea9952c97ca7c521c3a867fdf08dd82f1e1828b2cfa`.
Instruction pre-gate images.json binds exact M `f5822aa3…` and candidate
`0847b950…`; the candidate exactly matches the static matrix's R0 executable.
Policy SHA-256 is `be123f7c5cfd46aa863467256eb8bf6947ff64f72541efc371f05a36c5535a31`.

## Instruction pre-gate: BLOCK; stop before timing

The complete fixed-policy run (one pair per case, the existing T056
instruction-collector recipe) has 86/86 unique cases, no missing instruction
sample and 46 over `max(1.03, I_i)`. Every cutoff here is 1.03. Broad fails
14/25, sentinel 5/6, external 25/45 and development 2/10. All three declared
targets pass this instruction-only gate; controls still block the candidate.
This is instruction evidence only: incidental wall/cycle fields from the
shared collector are not a timing screen, K=6 panel or formal comparison.
The pre-gate used exactly M/candidate image hashes in the preceding section;
its candidate is byte-identical to the qualified static R0 image.

| Receipt | SHA-256 |
| --- | --- |
| images.json | `039dd530e8571ddfd8bb387d6fc4152bcf2e1a4aa96078a3f3605645bbead5b1` |
| pregate.json | `9d7116aefe98b2c96e259893c3e0bba057031203c1dfb661ccc9051f8a2888ad` |
| verdict.json | `62794c4e3eb39630823bb9c00f1fe90997347a3e65df99873ef4899cae5e9e48` |

Per-case candidate/M instruction ratios below are the entire frozen pre-gate
record, ordered worst first. No case, ratio, threshold, image or sample was
selected or changed after seeing the result.

| Policy case | Instructions candidate/M | Cutoff | Result |
| --- | ---: | ---: | --- |
| `development/big.js/arith` | 1.642905896 | 1.03 | BLOCK |
| `external/sunspider-1.0/access-nbody` | 1.483719559 | 1.03 | BLOCK |
| `broad/math_abs` | 1.452852004 | 1.03 | BLOCK |
| `sentinel/string_key_map_churn` | 1.444384322 | 1.03 | BLOCK |
| `external/kraken-1.1/imaging-gaussian-blur` | 1.402730300 | 1.03 | BLOCK |
| `external/kraken-1.1/ai-astar` | 1.350275933 | 1.03 | BLOCK |
| `external/sunspider-1.0/math-partial-sums` | 1.296599898 | 1.03 | BLOCK |
| `broad/method_call` | 1.295137264 | 1.03 | BLOCK |
| `external/sunspider-1.0/crypto-aes` | 1.294413501 | 1.03 | BLOCK |
| `external/sunspider-1.0/access-fannkuch` | 1.288493554 | 1.03 | BLOCK |
| `broad/top_level_function_call` | 1.273296896 | 1.03 | BLOCK |
| `broad/plain_function_call` | 1.257590923 | 1.03 | BLOCK |
| `broad/function_call_reordered` | 1.257543303 | 1.03 | BLOCK |
| `broad/function_call_two_args` | 1.253945208 | 1.03 | BLOCK |
| `broad/captured_read` | 1.250243161 | 1.03 | BLOCK |
| `sentinel/polymorphic_call_site` | 1.249768263 | 1.03 | BLOCK |
| `sentinel/heterogeneous_property_read` | 1.246511896 | 1.03 | BLOCK |
| `external/kraken-1.1/imaging-darkroom` | 1.245709355 | 1.03 | BLOCK |
| `sentinel/capturing_closure_call` | 1.238739462 | 1.03 | BLOCK |
| `external/sunspider-1.0/crypto-sha1` | 1.220858504 | 1.03 | BLOCK |
| `external/sunspider-1.0/math-spectral-norm` | 1.211000234 | 1.03 | BLOCK |
| `sentinel/prototype_method_call` | 1.208430956 | 1.03 | BLOCK |
| `external/kraken-1.1/stanford-crypto-aes` | 1.200247276 | 1.03 | BLOCK |
| `external/sunspider-1.0/3d-morph` | 1.199887344 | 1.03 | BLOCK |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 1.191302534 | 1.03 | BLOCK |
| `broad/array_index_of` | 1.169883956 | 1.03 | BLOCK |
| `external/sunspider-1.0/access-nsieve` | 1.151675999 | 1.03 | BLOCK |
| `external/sunspider-1.0/3d-cube` | 1.148906559 | 1.03 | BLOCK |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | 1.145240928 | 1.03 | BLOCK |
| `external/kraken-1.1/audio-dft` | 1.124151579 | 1.03 | BLOCK |
| `development/matter-js/simulate` | 1.120449449 | 1.03 | BLOCK |
| `broad/many_locals_call` | 1.120033654 | 1.03 | BLOCK |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | 1.099581322 | 1.03 | BLOCK |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | 1.086965577 | 1.03 | BLOCK |
| `broad/string_slice` | 1.083016008 | 1.03 | BLOCK |
| `external/sunspider-1.0/crypto-md5` | 1.081624828 | 1.03 | BLOCK |
| `external/sunspider-1.0/math-cordic` | 1.068394038 | 1.03 | BLOCK |
| `external/sunspider-1.0/bitops-bits-in-byte` | 1.067197420 | 1.03 | BLOCK |
| `broad/local_read` | 1.066076508 | 1.03 | BLOCK |
| `external/kraken-1.1/audio-fft` | 1.065897893 | 1.03 | BLOCK |
| `broad/global_read` | 1.065796584 | 1.03 | BLOCK |
| `broad/empty_loop` | 1.065484456 | 1.03 | BLOCK |
| `broad/branch_arithmetic` | 1.065221362 | 1.03 | BLOCK |
| `external/kraken-1.1/stanford-crypto-ccm` | 1.058706722 | 1.03 | BLOCK |
| `external/sunspider-1.0/bitops-bitwise-and` | 1.055903823 | 1.03 | BLOCK |
| `external/kraken-1.1/audio-beat-detection` | 1.047254827 | 1.03 | BLOCK |
| `external/jetstream3-js-subset/hash-map` | 1.029307284 | 1.03 | PASS |
| `external/sunspider-1.0/string-validate-input` | 1.020972583 | 1.03 | PASS |
| `external/sunspider-1.0/controlflow-recursive` | 1.010529819 | 1.03 | PASS |
| `external/sunspider-1.0/date-format-xparb` | 1.008552828 | 1.03 | PASS |
| `external/sunspider-1.0/string-unpack-code` | 1.008516896 | 1.03 | PASS |
| `external/sunspider-1.0/string-tagcloud` | 1.008315155 | 1.03 | PASS |
| `development/meriyah/parse` | 1.006142213 | 1.03 | PASS |
| `development/graphlib/paths` | 1.004362883 | 1.03 | PASS |
| `sentinel/recursive_call_tree` | 1.003206692 | 1.03 | PASS |
| `development/papaparse/csv` | 1.002859600 | 1.03 | PASS |
| `development/moment/dates` | 1.002818640 | 1.03 | PASS |
| `external/sunspider-1.0/3d-raytrace` | 1.002495262 | 1.03 | PASS |
| `development/js-yaml/document` | 1.002261247 | 1.03 | PASS |
| `external/sunspider-1.0/bitops-nsieve-bits` | 1.001648415 | 1.03 | PASS |
| `development/lodash/transform` | 1.001415054 | 1.03 | PASS |
| `development/diff/compare` | 1.000902097 | 1.03 | PASS |
| `external/sunspider-1.0/regexp-dna` | 1.000628080 | 1.03 | PASS |
| `external/jetstream3-js-subset/gaussian-blur` | 1.000505893 | 1.03 | PASS |
| `broad/array_dynamic_read` | 1.000220056 | 1.03 | PASS |
| `external/kraken-1.1/audio-oscillator` | 1.000038548 | 1.03 | PASS |
| `external/kraken-1.1/imaging-desaturate` | 0.999770291 | 1.03 | PASS |
| `external/kraken-1.1/json-stringify-tinderbox` | 0.999672796 | 1.03 | PASS |
| `development/immutable/persistent` | 0.999671063 | 1.03 | PASS |
| `external/sunspider-1.0/date-format-tofte` | 0.998408870 | 1.03 | PASS |
| `external/sunspider-1.0/string-fasta` | 0.998357123 | 1.03 | PASS |
| `external/kraken-1.1/json-parse-financial` | 0.997398578 | 1.03 | PASS |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.997379285 | 1.03 | PASS |
| `external/sunspider-1.0/string-base64` | 0.995589850 | 1.03 | PASS |
| `external/jetstream3-js-subset/cdjs` | 0.995288525 | 1.03 | PASS |
| `broad/property_dynamic_read` | 0.995287114 | 1.03 | PASS |
| `broad/object_allocation` | 0.992817841 | 1.03 | PASS |
| `broad/array_allocation` | 0.992671175 | 1.03 | PASS |
| `broad/dynamic_method_call` | 0.990811819 | 1.03 | PASS |
| `broad/closure_allocation_call` | 0.990348146 | 1.03 | PASS |
| `broad/property_read` | 0.990143106 | 1.03 | PASS |
| `broad/array_read` | 0.989786949 | 1.03 | PASS |
| `broad/property_write` | 0.987178021 | 1.03 | PASS |
| `broad/array_write` | 0.986950610 | 1.03 | PASS |
| `external/sunspider-1.0/access-binary-trees` | 0.984569906 | 1.03 | PASS |
| `broad/captured_write` | 0.983813418 | 1.03 | PASS |

## Handoff and limits

Stop/report is the brief's authorized outcome for a failed instruction
pre-gate. No repair, repeated pre-gate, hint/padding/inline search, relaxed
threshold or timed experiment follows this result. The extra instructions
are confirmed; their division among service-call overhead, lost inlining,
register allocation and unrelated hot-body codegen effects has not been
measured, so no causal fraction is claimed. Zero services in the scalar
regression test does not claim zero total instruction cost from outlining.

K has spent 0 of its at-most-one failed **timed screen** allowance; the old
A+B failure remains spent and its one shared reserve remains unchanged.
The candidate is not admitted to screening or stage advance. No full exact
configured Test262 scan, formal resource/RSS cohort or performance standing
is claimed; these later stage-decision requirements cannot be waived.
No dependency, crate, FFI, unsafe, recipe, tier-order, source/benchmark
recognition or binding-model change was introduced. Static coverage is
finite and Darwin x86_64-specific; observer source tests cover script and
function execution while forced per-arm stack/PC tests instantiate Vm.
Existing wide deopt/receiver/stack tests pass, and both compiled roles pass
the complete matrix, but no per-arm forced Wide frame harness was added.
The local disk guard stayed above 8 GB; final `target/debug` was removed
and available space returned to about 69 GiB. The assigned worktree/branch
remain, with no branch push, merge or removal. This record is a documentation
follow-up to runtime candidate `671e9943`; no runtime edit followed measurement.

## Repair 1: false cold hint

Lead feedback 1 authorizes exactly one change: remove `#[cold]` from
`service::execute`, retaining its `#[inline(never)]`. Exit publication,
deoptimization and materialization remain cold/non-inlined. The lead's
hypothesis is that a frequently executed service's false cold hint lowers
LLVM's helper-inlining threshold; it is a hypothesis until attributed.
No other runtime hint, attribute or structure changes in this round.
Four cases and all three images are fixed before collection: big.js/arith,
access-nbody, math_abs, string_key_map_churn; M `9eba7a1d`, failed `671e9943`,
and the repair commit. Attribution completed before the new pre-gate. Static
and correctness pass; the instruction pre-gate still blocks. This round is
finished; no further repair follows.

### Repair implementation, commands and measurement discipline

Repair candidate `773c42f7` has one runtime attribute deletion relative to
`671e9943` (plus its module-header wording); no other runtime source, hint,
layout, tier/admission, recipe or frozen-plan edit. Exit attributes are
unchanged. Architecture/design wording now calls the service non-inlined.
The attribution producer is [repair-attribution.py](../diagnostics/t064-k1/repair-attribution.py).

Local full `check.sh` passes for that candidate's source, including 2,342
runtime tests, agents/perf-counter checks, 723 benchmark-tool tests, 35 script
tests and 5,170/5,170 curated cases. Fast gate passes with its 65 selected
semantic cases; NG passes 256/256. Local log hashes: full
`2fa2dca34f961a454f9f62c5aa153196079b9c699f5274beac66f26105996dc3`, fast
`2228c11d1e68a939c3bd41d4dc5b6f64e5000185f24d877a786fcf509c61336b`, NG
`9f2dc6f7fe1468d6f4175c1e9828d6823c1b2ce0e515d1ab8334b868917bbb5f`.

All queue jobs omit `--timing`:

- `20261022T070342Z-t064-k1-repair1-attribution`: canonical repair build
  succeeds, then the file-script launcher lacks PYTHONPATH and exits 1 before
  collecting any profile or workload counter. Retained; no data selected.
- `20261022T070413Z-t064-k1-repair1-attribution-launch`: same candidate and
  predeclared collection, explicit `PYTHONPATH="$PWD"`; exits 0 after 390 s
  of diagnostic work, all 12 records complete before the pre-gate. No repair
  source or measurement parameter was changed for the launcher correction.
- `20261022T070427Z-t064-k1-repair1-static`: full matrix PASS, exits 0 after 423 s of build/check work.
- `20261022T070510Z-t064-k1-repair1-pregate`: exits 3 after 199 s of diagnostic
  work, 46/86 instruction cases blocked. Its launcher verified the completed
  4-case/3-image/12-record attribution SHA and exact repaired revision before
  invoking the collector.

Canonical image hashes: M `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb`,
old cold `0847b950b2279dbf194cca7bdf234e87b1b8410ea38f6177e64fa458e3b67a30`,
repair `f64ee3e65b5f86291638468cef1796bb998e45ac0695345cdc372d57bb040797`.
Whole-process instructions below are one original workload launch per image,
using existing `run_process`/zombie hardware counters. Internal N is fixed
at 100,000,000 for math_abs and 10,000,000 for map churn. Samples are separate
runs: original internal workloads at those N, external/development bundle
bodies repeated 40 times in a generic outer loop, sample at 1 ms for up to
8 seconds with `-mayDie`. Repetition keeps short processes observable; it is
not the original whole-process instruction program, and sample shares must
not be multiplied into those instruction counts or called wall speedups.
The earliest 8-second window is not a complete-process phase census.

The tree parser conserves every main-thread self sample and separates the
service body, descendants of the service, the exact protected hot body,
other descendants of that body, driver/caller self work and everything else.
M's inline object operations are charged to its original hot `run` closure;
M has no service symbol. Caller covers wide activation/run_typed_loop_here
and the VM driver, not the hot body's callees. The parser was first replayed
on the existing exact-M moment profile (8,288 samples conserved).
After fetching, all 12 binary/process/raw-sample/receipt/workload hash
bindings and sample-count partitions were independently checked locally.

| Attribution artifact | SHA-256 |
| --- | --- |
| attribution-request.json | `baf2fa139aad747f3141ad416a1a81a9cff3a93356ba978197f07ab969346f81` |
| attribution.json | `ebf5be9f4b7874a4fe9aff823df0bf53155bc1cc657198180f5bc3c7e8553ac9` |
| attribution-complete.json | `4f1293b1634698d20369e125b5e2efabb88249beb15367992798994292f1fc07` |

### Repair whole-process instructions and symbol attribution

| Case | M instructions | Cold instructions | Repair instructions | Cold/M | Repair/M |
| --- | ---: | ---: | ---: | ---: | ---: |
| `development/big.js/arith` | 24,388,764,218 | 40,065,867,341 | 39,898,067,545 | 1.642800225 | 1.635920016 |
| `external/sunspider-1.0/access-nbody` | 315,475,945 | 468,035,139 | 470,982,851 | 1.483584237 | 1.492927935 |
| `broad/math_abs` | 34,057,791,184 | 49,469,209,258 | 49,869,627,999 | 1.452507856 | 1.464264894 |
| `sentinel/string_key_map_churn` | 15,093,640,436 | 21,783,535,162 | 21,783,516,583 | 1.443226056 | 1.443224825 |

Self-sample percentages of the recorded main-thread total (not instruction
percentages); all six columns partition the sample total:

| Case | Image | Samples | Service body | Service callees | Hot body | Hot callees | Caller | Other |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `development/big.js/arith` | M | 6651 | 0.00% | 0.00% | 57.81% | 21.94% | 8.03% | 12.22% |
| `development/big.js/arith` | cold | 6663 | 36.43% | 18.10% | 32.34% | 0.81% | 4.95% | 7.37% |
| `development/big.js/arith` | repair | 6655 | 35.07% | 16.80% | 34.08% | 0.98% | 5.26% | 7.81% |
| `external/sunspider-1.0/access-nbody` | M | 808 | 0.00% | 0.00% | 66.46% | 11.51% | 10.02% | 12.00% |
| `external/sunspider-1.0/access-nbody` | cold | 1194 | 24.04% | 14.15% | 41.29% | 2.76% | 10.05% | 7.71% |
| `external/sunspider-1.0/access-nbody` | repair | 1203 | 24.85% | 13.97% | 40.23% | 2.58% | 8.56% | 9.81% |
| `broad/math_abs` | M | 2084 | 0.00% | 0.00% | 86.76% | 13.24% | 0.00% | 0.00% |
| `broad/math_abs` | cold | 3119 | 38.63% | 9.39% | 51.94% | 0.00% | 0.00% | 0.03% |
| `broad/math_abs` | repair | 3057 | 36.28% | 8.96% | 54.73% | 0.00% | 0.00% | 0.03% |
| `sentinel/string_key_map_churn` | M | 1003 | 0.00% | 0.00% | 53.14% | 46.76% | 0.00% | 0.10% |
| `sentinel/string_key_map_churn` | cold | 1378 | 30.70% | 37.59% | 31.71% | 0.00% | 0.00% | 0.00% |
| `sentinel/string_key_map_churn` | repair | 1363 | 27.59% | 39.03% | 33.31% | 0.00% | 0.00% | 0.07% |

The symbol-level observations are:

- **big.js:** M is led by the Wide hot closure (3,845 self samples), then
  Value clone 471, drop 369 and direct dense reads 303. Cold splits that
  into service 2,427/hot 2,155; service leaves include clone 404, drop 241,
  dense_write_value 211 and direct dense reads 192. Repair remains service
  2,333/hot 2,268, with clone 323, drop 246, dense reads 201 and
  dense_write_value 181. Caller self work is a small share in all images.
- **nbody:** M's Wide hot closure is 537/808 samples. Cold and repair put
  287/299 in the service and 493/484 in hot dispatch. The named-slot helpers
  shared_data_slot_number have 48/53 and shared_data_slot_write_number 35/37
  under the cold/repaired services. Caller shares stay about 9–10%.
- **math_abs:** M's hot closure is 1,808/2,084 samples; math_unary already
  exists out of line (103), alongside clone 92/drop 81. Cold and repair still
  reach math_unary (111/109), clone (104/98) and drop (78/67). Hot dispatch
  and service self work dominate both; there is no material caller share.
- **map churn:** M's hot closure is 533/1,003; own_data_property_read 104,
  clone 92, write_existing_own_data_property 86 and memcmp 72. Cold/repair
  keep hot dispatch 437/454 and service 423/376. Their callee leader is
  DynamicPropertyStorage::property_mut (156/151), then clone (102/86),
  memcmp (81/72), drop (49/58), existing-write (38/53), own-data-read (31/31).
  Caller/other is negligible in this profile.

The recorded execute-owned symbol extents are identical cold→repair except
both hot bodies shrink 16 B (Vm 6,320→6,304; Wide 6,272→6,256). Both service
bodies remain 5,856/5,840 B and dense_write_value remains 352 B. These are
symbol extents, not a byte-equality or identical-inlining proof, but there
is no large service/helper size expansion corroborating the proposed repair.

A supplemental local arm64 agents+perf-counters run of the **same repair
source**, math_abs N=10,000, completes with checksum 10,000 and one typed
entry. Its program has exactly two service ops per accelerated iteration:
MoveBoxed and CallNumericNative; 19,998 services for 9,999 native iterations,
zero service deopts, one scratch allocation/growth, 4,176 B retained and
80 B live boxed scratch. This is work-count evidence, not x86 instruction or
timing attribution. The trace/log is local, not retained; its conclusion and
counts are recorded here.

**Attribution conclusion:** extra whole-process instructions are confirmed
and remain after removing the cold hint. Samples place the dominant work
inside the split typed executor (hot dispatch plus service body/callees),
with the named dense/slot/native/value helpers above; the caller is not the
principal sampled owner. Math's native helper was already out of line on M.
The cold-hint hypothesis therefore does not explain most of the excess in
these cases. The per-iteration non-inlined service work remains. Sampling
cannot apportion exact additional retired instructions among dispatch,
argument transport, helper calls and spills; no causal fraction is invented,
and no further repair is authorized or attempted this round.

Supplemental math work-count log SHA-256: `1b8b0e2ed1cbdd8e8b3257afb43a2bc516290f4d652c44d9dc8e34434a75b2f4`.

### Repair static matrix

Same unmodified checker/contract and canonical recipe as the original K1
matrix. Both R0 builds exactly reproduce the sampled/countered repair image
`f64ee3e6…`. Layout witnesses remain R0 112/400 B, H 120/400 B, C 112/408 B.
Both hot bodies retain their boundaries/relative PCs, non-address bytes,
three owned tables and stack reports under both perturbations: Vm
6,304 B/1,344 instructions; Wide 6,256 B/1,329 instructions. Allowed fixups
are reported rather than counted as raw-byte equality.

| Comparison | Vm | WideLoopFrame |
| --- | --- | --- |
| A/A | PASS, 0 failures, 0 allowed | PASS, 0 failures, 0 allowed |
| H+8 | PASS, 0 failures, 82 allowed | PASS, 0 failures, 25 allowed |
| C+8 | PASS, 0 failures, 25 allowed | PASS, 0 failures, 25 allowed |

| Image/receipt | SHA-256 |
| --- | --- |
| r0a | `f64ee3e65b5f86291638468cef1796bb998e45ac0695345cdc372d57bb040797` |
| r0b | `f64ee3e65b5f86291638468cef1796bb998e45ac0695345cdc372d57bb040797` |
| h8 | `4e7c6d45a6f0d0c75eeae063464672b55fb6f7ebb92b47a299f1f849543293e3` |
| c8 | `7dc044114aa4f16e491a4ca296af5d5d4ad2f5a8bba918919e389aa802b3cd08` |
| request.json | `72628eff41b3ba4798d3d99c4c7ce5bab1d18a6a05a036b2d33eb49a331a1e0d` |
| builds.json | `e8556b1b5bd202a5eac254cd98b9ffcd9564883e97891e590c5859f25eaeba85` |
| comparisons.json | `16c145a934994a82733fff86f246e46bf6f203efeb15c0c51cfef4241c71d26c` |
| result.json | `640632c0347c7769a263d2ef5e1a16543ae5bc5a7780aeb847eec586dd1ca363` |

Local unmodified checker replay verifies all four repaired images and
perturbation-source hashes, then reproduces comparisons.json exactly.
The candidate used in the repaired static matrix, attribution and pre-gate
has one identical executable SHA-256 across all three jobs.

### Repair full instruction pre-gate and final stop

The pre-gate completes all 86 unique policy cases, with no missing instruction
sample. Replaying every ratio, cutoff and verdict locally against pregate.json
and the unchanged policy reproduces the verdict: **BLOCK, 46/86**, with the
same 46 failing case identities as original K1. Broad fails 14/25, sentinel
5/6, external 25/45 and development 2/10. Each cutoff is still 1.03. The
whole-process four-case attribution above is a separate diagnostic launch
cohort; these are the fixed-policy pre-gate ratios used to block timing.

| Repair pre-gate artifact | SHA-256 |
| --- | --- |
| images.json | `519cb0bcdccc3243decbd372d01a27013c433fbaced5242d83c11b7f3d6758b4` |
| pregate.json | `c8efbca42cac5278475f6a0fd3a26ec639ad79f1d8bca96486fd73fc0393e740` |
| verdict.json | `505c5a3c6a9bbcb9cf3dd659719bec156ab2f62bfff5d38b6f8989c6857a4e83` |

| Policy case | Repair/M instructions | Cutoff | Result |
| --- | ---: | ---: | --- |
| `development/big.js/arith` | 1.636072916 | 1.03 | BLOCK |
| `external/sunspider-1.0/access-nbody` | 1.492531957 | 1.03 | BLOCK |
| `broad/math_abs` | 1.464677958 | 1.03 | BLOCK |
| `sentinel/string_key_map_churn` | 1.444209439 | 1.03 | BLOCK |
| `external/kraken-1.1/imaging-gaussian-blur` | 1.413206790 | 1.03 | BLOCK |
| `external/kraken-1.1/ai-astar` | 1.350182700 | 1.03 | BLOCK |
| `external/sunspider-1.0/math-partial-sums` | 1.306311925 | 1.03 | BLOCK |
| `broad/method_call` | 1.299457535 | 1.03 | BLOCK |
| `external/sunspider-1.0/access-fannkuch` | 1.295730354 | 1.03 | BLOCK |
| `external/sunspider-1.0/crypto-aes` | 1.294035380 | 1.03 | BLOCK |
| `broad/top_level_function_call` | 1.275084911 | 1.03 | BLOCK |
| `broad/plain_function_call` | 1.264707431 | 1.03 | BLOCK |
| `broad/function_call_reordered` | 1.264674452 | 1.03 | BLOCK |
| `external/kraken-1.1/imaging-darkroom` | 1.263814508 | 1.03 | BLOCK |
| `broad/function_call_two_args` | 1.260949408 | 1.03 | BLOCK |
| `sentinel/heterogeneous_property_read` | 1.258443019 | 1.03 | BLOCK |
| `broad/captured_read` | 1.257476666 | 1.03 | BLOCK |
| `sentinel/polymorphic_call_site` | 1.256937786 | 1.03 | BLOCK |
| `sentinel/capturing_closure_call` | 1.248975129 | 1.03 | BLOCK |
| `external/sunspider-1.0/3d-morph` | 1.222371486 | 1.03 | BLOCK |
| `external/sunspider-1.0/crypto-sha1` | 1.221482066 | 1.03 | BLOCK |
| `external/sunspider-1.0/math-spectral-norm` | 1.215456762 | 1.03 | BLOCK |
| `sentinel/prototype_method_call` | 1.212191541 | 1.03 | BLOCK |
| `external/kraken-1.1/stanford-crypto-aes` | 1.204229391 | 1.03 | BLOCK |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 1.194973314 | 1.03 | BLOCK |
| `broad/array_index_of` | 1.172258828 | 1.03 | BLOCK |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | 1.169624276 | 1.03 | BLOCK |
| `external/sunspider-1.0/access-nsieve` | 1.159460867 | 1.03 | BLOCK |
| `external/sunspider-1.0/3d-cube` | 1.156642649 | 1.03 | BLOCK |
| `external/kraken-1.1/audio-dft` | 1.123567521 | 1.03 | BLOCK |
| `development/matter-js/simulate` | 1.123550125 | 1.03 | BLOCK |
| `broad/many_locals_call` | 1.123425840 | 1.03 | BLOCK |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | 1.113210954 | 1.03 | BLOCK |
| `broad/local_read` | 1.111586171 | 1.03 | BLOCK |
| `broad/global_read` | 1.105284144 | 1.03 | BLOCK |
| `broad/branch_arithmetic` | 1.104514506 | 1.03 | BLOCK |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | 1.102042914 | 1.03 | BLOCK |
| `broad/empty_loop` | 1.093306658 | 1.03 | BLOCK |
| `broad/string_slice` | 1.084614940 | 1.03 | BLOCK |
| `external/sunspider-1.0/crypto-md5` | 1.082504836 | 1.03 | BLOCK |
| `external/sunspider-1.0/math-cordic` | 1.081969532 | 1.03 | BLOCK |
| `external/sunspider-1.0/bitops-bitwise-and` | 1.079505622 | 1.03 | BLOCK |
| `external/sunspider-1.0/bitops-bits-in-byte` | 1.068387318 | 1.03 | BLOCK |
| `external/kraken-1.1/audio-beat-detection` | 1.067195837 | 1.03 | BLOCK |
| `external/kraken-1.1/audio-fft` | 1.067084818 | 1.03 | BLOCK |
| `external/kraken-1.1/stanford-crypto-ccm` | 1.060294587 | 1.03 | BLOCK |
| `external/jetstream3-js-subset/hash-map` | 1.029110528 | 1.03 | PASS |
| `external/sunspider-1.0/string-validate-input` | 1.020946819 | 1.03 | PASS |
| `external/sunspider-1.0/controlflow-recursive` | 1.010179055 | 1.03 | PASS |
| `external/sunspider-1.0/string-tagcloud` | 1.009411731 | 1.03 | PASS |
| `development/meriyah/parse` | 1.008735984 | 1.03 | PASS |
| `external/sunspider-1.0/date-format-xparb` | 1.008385697 | 1.03 | PASS |
| `external/sunspider-1.0/string-unpack-code` | 1.008319548 | 1.03 | PASS |
| `development/graphlib/paths` | 1.004570683 | 1.03 | PASS |
| `development/js-yaml/document` | 1.004415577 | 1.03 | PASS |
| `sentinel/recursive_call_tree` | 1.003308772 | 1.03 | PASS |
| `development/moment/dates` | 1.002982162 | 1.03 | PASS |
| `external/sunspider-1.0/3d-raytrace` | 1.002374248 | 1.03 | PASS |
| `development/lodash/transform` | 1.002292364 | 1.03 | PASS |
| `external/sunspider-1.0/bitops-nsieve-bits` | 1.001482001 | 1.03 | PASS |
| `development/diff/compare` | 1.000683083 | 1.03 | PASS |
| `external/sunspider-1.0/regexp-dna` | 1.000534582 | 1.03 | PASS |
| `external/kraken-1.1/json-stringify-tinderbox` | 1.000268050 | 1.03 | PASS |
| `broad/array_dynamic_read` | 0.999978468 | 1.03 | PASS |
| `development/immutable/persistent` | 0.999897641 | 1.03 | PASS |
| `external/kraken-1.1/audio-oscillator` | 0.999839399 | 1.03 | PASS |
| `external/kraken-1.1/imaging-desaturate` | 0.999508534 | 1.03 | PASS |
| `external/sunspider-1.0/date-format-tofte` | 0.998778652 | 1.03 | PASS |
| `external/jetstream3-js-subset/gaussian-blur` | 0.998508548 | 1.03 | PASS |
| `external/sunspider-1.0/string-fasta` | 0.998331860 | 1.03 | PASS |
| `development/papaparse/csv` | 0.998171059 | 1.03 | PASS |
| `external/kraken-1.1/json-parse-financial` | 0.997306898 | 1.03 | PASS |
| `external/sunspider-1.0/string-base64` | 0.995443164 | 1.03 | PASS |
| `external/jetstream3-js-subset/cdjs` | 0.995374341 | 1.03 | PASS |
| `broad/property_dynamic_read` | 0.994984422 | 1.03 | PASS |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.994657449 | 1.03 | PASS |
| `broad/array_allocation` | 0.992901871 | 1.03 | PASS |
| `broad/object_allocation` | 0.992713799 | 1.03 | PASS |
| `broad/dynamic_method_call` | 0.991019237 | 1.03 | PASS |
| `broad/closure_allocation_call` | 0.990292167 | 1.03 | PASS |
| `broad/property_read` | 0.990116361 | 1.03 | PASS |
| `broad/array_read` | 0.989940150 | 1.03 | PASS |
| `broad/property_write` | 0.987138782 | 1.03 | PASS |
| `broad/array_write` | 0.987046398 | 1.03 | PASS |
| `external/sunspider-1.0/access-binary-trees` | 0.984434107 | 1.03 | PASS |
| `broad/captured_write` | 0.983558765 | 1.03 | PASS |

One authorized repair round is complete and stops here. Removal of the false
cold hint does not admit K to a timed screen; the complete attribution is the
deliverable. No second repair, hint/inline/padding search, threshold change,
new timing/K=6/formal run, frozen-plan edit, branch push or merge was performed.
K has still spent zero timed-screen failures, and the old A+B ledger remains
unchanged. Exact per-symbol retired-instruction deltas/spill attribution
remain unresolved; the measured excess and its sampled execution owners are
recorded without claiming a precise causal split. Source remains implemented
on the branch at `773c42f7`; this final follow-up changes evidence/resume docs
only. Full/fast/NG checks above apply to that exact runtime source. Final
context/whitespace/doc fast gate passes; target/debug is absent and local
free space is about 69 GiB. Return to the lead; no further work is authorized
by this one-round decision.
