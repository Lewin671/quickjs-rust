# T056 M1 stage A: implementation record

Stage A of plan [`interpreter-core-v2-layout-edges`](../performance-units/interpreter-core-v2-layout-edges.json)
(plan commit `50b42547`), implemented on `agent/t056-m1-stage-a/claude` per
[property layouts](../../docs/design/property-layouts.md). Owner of status and next action:
[T056](../T056-interpreter-core-v2.md). Numbers here are local diagnostics, not decision evidence.

## Branch content

Runtime commits over `50b42547`, oldest first: `3506f4db` (layout authority for ordinary stores), `b1686896`
(class-field keys interned at definition, lazy native default prototypes), `1979fcb7` (function and array named
tables), `83601f83` and its revert `a31e87fa` (see the disclosure below), `9d5f96fb` (slot writes through the
payload descriptor), `0abd7976` (one probe per named-table add, layout-owned function key order, shared fallback
domain), `7d8f18db` (slot accesses without reading the layout node), `9049dfc6` (a missed named write is the
creation's only probe), then tests and documentation.

Deviations from the design, with their reasons, are now stated in the design itself: the shared-slot dictionary
token (a `Weak` of the node plus the owner's layout revision), edit edges capped at 8 with content-cache hits
recording no edge, the lazy large-shape index, the per-thread shared fallback domain, and unpacked flags (packing
saves no bytes at 120).

## Disclosure: write-site add replay (removed from stage A)

`83601f83` remembered, per named write site with a creation proof and per public class field, the layout
transition of the last plain creation, and replayed it by layout identity on the next receiver. The review ruled it
outside stage A (a route, stage B's accelerated Set, or an L4 leaf for the class-field part, which conflicts with
"class fields and constructor stores cache no final shape"), so `a31e87fa` reverts it; it is kept on
`agent/t056-add-transition-replay/claude`. It was implemented and measured during stage A. Local arm64 instruction
ratios against the base image of `50b42547` (min of 3, whole process):

| Workload | `1979fcb7` (before) | `83601f83` (with replay) |
| --- | ---: | ---: |
| binary-trees, 400 passes | 1.0536 | 0.8558 |
| constructor `new Node(l, r, v)`, 3M | 1.0344 | 0.9437 |
| class fields `new P()`, 4 fields, 2M | 1.0194 | 0.9762 |
| raytrace-class-fields bundle | 1.0203 | 0.9572 |
| 3d-raytrace bundle | 0.9825 | 0.9819 |
| `new Object()`-style empty construction | 1.0022 | 1.0022 |

These ratios include the replay; the stage A numbers below do not.

## Local instruction ratios at `9049dfc6` (arm64, min of 3, against the base image)

| Workload | Ratio |
| --- | ---: |
| raytrace-class-fields bundle | 1.0234 |
| binary-trees, 400 passes | 1.0177 |
| class fields, 4 fields | 1.0155 |
| constructor, 3 fields | 0.9911 |
| moment/dates, diff/compare, meriyah/parse (single passes) | 0.9880, 0.9764, 0.9764 |
| 3d-raytrace, JSON.parse loop, empty construction | 0.9888, 0.9915, 0.9951 |

Broad screen at `7d8f18db` (local, increment-measured): `top_level_function_call` 0.9919 (1.0251 before the
store kept its encoding and the dictionary token), `array_index_of` 0.9953, `captured_write` 0.9858,
`global_read` 1.0001. `array_index_of` had no hot-path source change; its per-iteration perf-counter events
equal the base's (none counted), and arm64 `wide::activation::run` is 4596 instructions on the base and 4520
here with the same callee set except two slot readers that changed inlining, so its earlier 1.028 was code
placement. The x86 symbol comparison rides on the pre-gate image.

Startup (`0;`, whole process): 21.05M base, 21.29M here (+1.1%); atom domains at startup fell from 10 to 2 (the
realm's and the shared fallback). The remaining cost is the append chains of large install objects: 658 shapes and
11,098 field copies per realm.

## Gates

At `ccbc2378` plus the documentation commit: `./scripts/check.sh` passed (format, clippy for default, agents and
perf-counters, all three test configurations, benchmark tool tests, plans, context, file sizes, Test262 subset of
5170 cases) and `./scripts/compare-qjs.sh` passed all 255 fixtures, including the three new layout fixtures.

## Pre-gate

Not run yet: it runs on the final committed image, after the full gate.
