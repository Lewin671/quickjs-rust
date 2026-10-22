# T064 causal outlining spike

This is a fixed, non-timing experiment on main06e199fe, not K1 or a production
optimization. `transform.py` moves whole non-scalar operation arms into one
`#[cold] #[inline(never)]` helper and prevents object-valued publication/deopt
from inlining. It also pins the execute boundary out of line so entry seeding
cannot be fused into it. This is broader than outlining only four brand loads;
no claim about its service cost is made. The unchanged arms preserve operation
order, exact failure site and iteration/admission rules.

`build.py` requires an `imac-bench run` job marked no-timing. It predeclares seven
same-path, independent clean canonical builds: r0a/r0b, h8/c8, o-r0/o-h8/o-c8.
Separate test builds measure layout; probe code is restored before release
builds. All other inputs come from the pinned archive. The normal fat16/abort/
agents recipe is imported unchanged. No executable cache reuse, timing,
performance decision, PMU helper, new crate or production source edit.

Submit the producer scripts as a payload through the queue; retain its
`request.json`, producer hashes, build/layout logs, images and source preimages.
The first two jobs (234712Z and234929Z) stopped at the CLI smoke check after
one R0 build: missing console, then an extra final undefined from print.
They are retained as failures, not A/A or perturbation evidence. The corrected
smoke uses the final expression value. The third job (235048Z) established identical A/A images, then stopped because
the H+8 generator missed ordered_data_builder.rs. All six unique source
variants subsequently passed local cargo check before the fourth job. Failed/
cancelled semantic jobs are recorded with the final result; only complete
matrices count. The complete job is recorded in T064.

On macOS, with the fetched artifacts and this repository at the task revision:

```sh
PYTHONPATH="$PWD" python3 tasks/diagnostics/t064-k-spike/compare.py \
  --input <job-out> --output <exclusive-audit-directory>
```

`decode.py` reuses the T056 finite-small-number/boolean selected-path method,
reading actual emitted tables. Original main's typed body is emitted as the
`run` closure; outlining emits `execute`. Select exactly one large structural
role per frame, compare only within each family, and fail on ambiguity.
The decoder resolves the actual ops-length loop-header comparison and Boolean
tag/payload branches. It retains raw bytes, relative PCs, table offsets/targets, RIP offsets/
8/16-byte referents, body bytes, spills, assumptions and original disassembly.
Instruction decoding errors fail; these selected paths are not a complete CFG
or a production isolation checker. Table bytes disassembled as instructions
outside the selected paths are data, not accepted as executable coverage.
Strict comparison masks no bytes. Any mnemonic/address-normalized attribution
is supplemental and cannot establish invariance. A finite pass cannot prove
all future compiler transformations safe; a failure cannot prove only FFI works.

`closure.py --input <job-out> --audit <audit-directory>` additionally aligns every
outlined-body instruction boundary and attributes all changed raw encodings to
literal/metadata references or structurally identified calls, including import
stubs from `otool -Iv`. It verifies unchanged constant section offsets and full
numeric operand contents. This attribution normalizes nothing for acceptance.
Entry prologue literals and numeric `_fmod`/`_pow` fixups belong to the protected
closure even when their loads/calls are outside the nine selected paths.
The [result record](../../archive/T064-causal-outlining-spike.md) owns conclusions,
exact section/function movements, image/receipt hashes and interrupted jobs.

To reproduce builds after this branch is committed, submit `build.py` from the
spike revision through `imac-bench run --ref <spike-revision> --with <base>`
without `--timing`, using `env PYTHONPATH=. python3 tasks/diagnostics/t064-k-spike/build.py`.
Run `semantics.py <job-out> <pinned-NG-binary> <new-job-out>/semantics.json` in a
separate non-timing queued job, then the generated source's agents runtime tests.
