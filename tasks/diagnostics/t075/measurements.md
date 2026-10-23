# T075 measurements at 59eb72e5

Method and identity: [collection/source audit](method.md). Decisions and limits:
[T075](../../T075-executor-anatomy.md). All percentages below use equally
weighted development programs and sampled main-thread **self** counts.
They are not percentages of retired instructions or removable wall time.

## Expanded execution bucket

The unchanged T068 v2 owner taxonomy gives **46.403618%** execution/loop now,
versus the historical 44.21% at `f49be95c`. Those measurements straddle T069
and use different samples; the difference is not a regression claim.
Selected executors: wide **25.945611%**, typed/WideLoopFrame **8.178772%**,
ordinary VM **4.753341%**, numeric compact family **0.434931%**. Typed/Vm has
no observed canonical self sample. Sum **39.312655%**, 84.72% of the new owner
bucket. The other **7.090963%** is already visible helper self time.

| Inside the six Rust executor symbols | % of main-thread samples | % within executors | Full-debug check, % of main-thread samples |
| --- | ---: | ---: | ---: |
| Opcode fetch, PC bounds, decoding and dispatch | 5.7042 | 14.51 | 5.6023 |
| Operand/register/Value transport and clearing | 7.9273 | 20.16 | 8.2442 |
| Value tags, primitive clone selection, conversion/truthiness | 3.9912 | 10.15 | 3.9104 |
| Identified inlined Rc count manipulation | 0.0366 | 0.09 | 0.0340 |
| Identified operand bounds compare/branches | 0.5090 | 1.29 | 0.3399 |
| Inlined named-property/cache probes and validation | 3.2790 | 8.34 | 3.3500 |
| Indexed access and its receiver/index guards | 4.0688 | 10.35 | 3.8058 |
| Local/upvalue routing and borrowed-cell access | 2.6477 | 6.73 | 2.6304 |
| Inlined activation/call/return preparation | 4.5870 | 11.67 | 4.5683 |
| Result/completion/exception plumbing | 0.2079 | 0.53 | 0.2633 |
| Typed-loop entry/residency/explicit guards | 0.1362 | 0.35 | 0.1403 |
| Arithmetic/comparison and scalar operation guards | 2.6452 | 6.73 | 2.4568 |
| Unresolved semantic context | 3.5726 | 9.09 | 4.2228 |
| **Total** | **39.3127** | **100.00** | **39.5687** |

The full-debug column is an independently sampled, codegen-different image;
it corroborates broad attribution and does not replace canonical counts.
Canonical executor-share equal-program means by repeat are 39.3830%, 38.9339%,
39.6185%; debug 39.8608%, 39.1151%, 39.7453%. These are repeat observations,
not independent-sample confidence limits.

Inlined RC is only **0.0366%** here. The separate owner taxonomy still assigns
**12.1578%** to visible ownership functions outside these executors. This
refutes treating every inline register move as a hidden Rc increment; it does
not show that total ownership is cheap. Similarly the 0.2079% completion row
includes successful completion tests and is not an exception-frequency count.
Identified dispatch is 5.7042%, not the whole 46.4036% bucket. Unresolved
executor contexts stay 3.5726%; PC checks in dispatch are not counted again
as operand bounds. Unattributed source contexts are not a reason to promise
an exact total dispatch cost.

## Source and instruction hotspots

Innermost source lines are compiler annotations. The full inline chain and
instruction for every PC are retained in pc-map/frames; line 0 is explicit.
Source paths below are relative to `crates/qjs-runtime/src/` unless noted.

| Canonical PC | Main-thread % | Instruction | Inline source/context |
| --- | ---: | --- | --- |
| `0x100100e8e` | 0.7670 | `movq -0x1f8(%rbp), %rcx` | std `mem::replace<Value>` → execute.rs:47 `store` → wide/activation.rs:774, Move/Dup publication |
| `0x100103c46` | 0.6912 | `movq -0x60(%rbp), %rcx` | replace → store → wide/activation.rs:442 `clear_window`, called at :1249 |
| `0x100100db5` | 0.6168 | `movq -0x60(%rbp), %rcx` | replace → store → wide run_frames:0; semantic caller line unresolved |
| `0x1001123de` | 0.5780 | `movq %rsi, %rcx` | vm.rs:539 opcode dispatch |
| `0x100102597` | 0.5619 | `cmpq %r15, %r12` | slice get<WideOp> → wide/activation.rs:755, PC bound |
| `0x10010353d` | 0.4798 | `movq -0x2b8(%rbp), %rcx` | replace → store → wide/activation.rs:926, numeric binary result publication |
| `0x1000ffd1e` | 0.4634 | `movq -0x60(%rbp), %rcx` | replace → store → wide/activation.rs:766, constant publication |
| `0x1000ffc40` | 0.4021 | `incq %r12` | wide/activation.rs:758, PC increment |
| `0x1000ffc30` | 0.3729 | `movq -0x1a0(%rbp), %rax` | wide run_frames:0, retained unresolved |
| `0x1000ffc4e` | 0.3586 | `addq %rcx, %rax` | wide/activation.rs:761, dispatch target construction |
| `0x1002627a1` | 0.3438 | `movq %rax, -0x108(%rbp)` | typed_loop/execute.rs:472, ComputedRead result handling |
| `0x1001123fa` | 0.3434 | `addq %r14, %rcx` | vm.rs:539, dispatch target construction |

Cross-cutting instruction/context reaches below **overlap** the category table
and one another; never sum them as independent costs:

- Direct stack-source `mov*`/`vmov*` at `%rbp`/`%rsp`: **8.2201%**, of which wide
  **6.3425%** and typed-wide **1.3384%**. Stack-destination moves: **1.9774%**.
  This prices sampled stack traffic, not a demonstrated cache/TLB/store-load stall.
- Inline `mem::replace<Value>` context: **4.3949%**; `clone_local_value`: **1.8010%**.
- `clear_window` context: **1.3844%**, including the transport it invokes.
- `probe_hot` context: **1.7509%**; shared live-slot read context **0.4962%**.
- `upvalue_cell` context: **0.7328%**, including received-index computation
  **0.4236%**; upvalue get/borrow context **0.5330%** is a separate contextual reach.
- Typed `computed_read` helper context: **0.4639%**; `dense_read` **0.2656%**;
  `element_read` **0.2346%**. The indexed category also includes their caller-side
  checks, decoding and result handling, hence its larger 4.0688%.

Largest other visible loop owners: wide `eval_binary` 0.3917%,
`run_typed_loop_here` 0.3596%, VM `op_load_local` 0.3403%, inherited
`memset_pattern16` 0.3355%, typed `get_named_object` 0.2630%, VM `run_rare_op`
0.2403%, `eval_binary_without_env` 0.2250%, wide `eval_unary` 0.2199%,
numeric-plan `run_leaf` 0.2164%, VM `run_general_op` 0.2151%.
[Owner-envelope JSON](owner-envelope.json) retains the complete 7.0910% tail.

## Development distribution

These are shares of each program's own main-thread N, not percentages of its
executor time. The full 13-category counts are in summary.json.

| Program | Executors % | Dispatch % | Transport % | Tags % | Indexed % | Bindings % | Calls % | Named/cache % |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| big.js/arith | 66.273 | 11.697 | 6.822 | 6.749 | 26.687 | 0.109 | 0.673 | 0.364 |
| diff/compare | 27.456 | 6.115 | 5.769 | 2.302 | 0.381 | 3.224 | 2.288 | 1.844 |
| graphlib/paths | 32.871 | 4.294 | 9.816 | 3.552 | 1.324 | 1.324 | 5.554 | 2.680 |
| immutable/persistent | 39.565 | 5.442 | 8.069 | 3.940 | 0.970 | 4.070 | 7.111 | 3.526 |
| js-yaml/document | 34.615 | 4.403 | 7.773 | 5.122 | 0.379 | 3.314 | 4.781 | 3.778 |
| lodash/transform | 54.599 | 4.473 | 15.105 | 6.104 | 2.700 | 4.501 | 11.533 | 1.266 |
| matter-js/simulate | 55.788 | 8.005 | 9.474 | 3.622 | 6.896 | 0.593 | 1.805 | 11.536 |
| meriyah/parse | 35.818 | 5.755 | 7.852 | 3.597 | 0.566 | 3.015 | 4.607 | 4.041 |
| moment/dates | 26.485 | 3.960 | 4.276 | 2.649 | 0.220 | 2.730 | 4.932 | 2.900 |
| papaparse/csv | 19.658 | 2.897 | 4.317 | 2.275 | 0.564 | 3.597 | 2.586 | 0.856 |

## NG comparison

Canonical NG `JS_CallInternal` self is **48.3885%** (three repeat means
48.3422%, 48.8730%, 47.9544%). Its normalized code matches the debug rebuild.
Meaningful observed inline families:

| Within NG JS_CallInternal | % of NG main-thread samples |
| --- | ---: |
| Local/argument/operand transport and publication | 6.4203 |
| Named/global/property lookup | 7.6742 |
| Calls and activation setup | 5.1981 |
| Inline duplicate/free/reference counts | 3.3055 |
| Explicit decode/dispatch/interrupt polling | 2.8742 |
| Arithmetic/comparison | 1.9313 |
| Bindings | 1.3017 |
| Separately visible tag tests | 0.2310 |
| Unresolved/other | 19.4523 |

NG macro-expanded `JS_CallInternal:0` alone owns **17.4224%**. It is not
silently relabelled dispatch. Inline `find_own_property` alone owns 3.7076%;
`JS_FreeValueRT` 1.9931%, `js_dup` 1.0607%, `set_value` 0.4586%, interrupt
polling 0.7502%. Both engines inline semantic work into their loops; the C
loop's larger self fraction does not mean more dispatch overhead. NG inlines
more Rc and property work while Rust exposes substantial out-of-line costs.
These are different implementations and denominators, not an apples-to-apples
cycle price or a causal decomposition of Rust's excess time. Zero classified
NG operand bounds/completion samples is not proof of no checks; macro source
attribution often cannot separate them. The brief's ~2.63x wall / ~2.1x
instruction ratios are historical context, not newly measured standings.

## Main-thread sample counts

| Program | Canonical Rust N | Full-debug Rust N | NG N |
| --- | ---: | ---: | ---: |
| big.js/arith | 5497 | 5390 | 2214 |
| diff/compare | 14423 | 13887 | 4826 |
| graphlib/paths | 3097 | 3133 | 1178 |
| immutable/persistent | 8452 | 8436 | 2536 |
| js-yaml/document | 10562 | 10680 | 2701 |
| lodash/transform | 3555 | 3469 | 1654 |
| matter-js/simulate | 7758 | 7727 | 3490 |
| meriyah/parse | 6533 | 6528 | 2089 |
| moment/dates | 25935 | 26373 | 7822 |
| papaparse/csv | 5143 | 5169 | 2672 |
| **Total** | **90,955** | **90,792** | **31,182** |
