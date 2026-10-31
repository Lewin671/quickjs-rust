# S2 post-order root-cause pass

Regenerated x86 order at 2cccf17d, arm64 untouched. Generator job
20261031T012158Z-t077-s2-r3-regenerate-x86-order uses canonical a8e7709f and
its defaults (external/sentinel/broad only). The first post-order job refused a
dirty runner before building (exit 98); the generator's verified output was
restored there after local commit. No measurement was discarded.

Retry 20261031T013328Z-t077-s2-r3-post-order-public86-r2 completes all86 with
strict linked-layout audit passing: [audit/image](T077-s2-r3-layout-audit.json),
[raw triage](T077-s2-r3-post-order.json). Canonical candidate SHA
7bf456320e881f4b8ddaa3a1ae35573044a23c481b98f3ebddc47b3a624c70c7;
base dcee3c56 SHA 6fc7251582f597f2e85cc8ce9e3a37cb7aaf9a32c442ab03a2abdaccb1204d3b.
Default wide offset 0x400, no offset search.

Natural-layout wall/instruction geomeans: development .957437/.945659,
external 1.023751/1.009613, broad .996810/.983115, sentinel .950461/.944313,
all86 1.002755/.989616; automatic likely loss. Diagnostic, no formal/held-out claim.

The five highest wall ratios among external/broad are fixed for one pass:
3d-raytrace 1.252653/1.103843; method_call 1.226258/1.002803;
hash-map 1.143203/.979855; gaussian-blur 1.121193/1.103223;
array_index_of 1.106032/.967572. Profile their exact triage images and final argv,
eight original-workload processes per role/case, sampling 1s at 1ms with mayDie.
No scaling/source change, development or held-out inputs.

[Collector](T077-s2-r3-profile-pass.py) records raw profile hashes, self sample
counts/approximate milliseconds, three separate unprofiled whole-case counter
runs and decoded static function instructions/extents. macOS sample does not
provide per-function dynamic retired instructions; static instructions are not
presented as those counts. Unqualified cycle/cache-miss counters stay unavailable.
Classification and any demonstrated structural correction follow the pass;
there is no new runtime change before that evidence.

## Classification and freeze

Job `20261031T014811Z-t077-s2-r3-five-case-root-profiles` completed all five
pairs. [Revision-bound summary](T077-s2-r3-profile-summary.json) retains exact
argv, image/source/sample hashes, counters and top self functions. Static counts
are corrected by decoded address within unique `nm` extents: printed `otool`
labels can include aliases/local labels and overcount a function. Aliases have
no claimed static count. Self counts below are accumulated approximately-ms
at 1ms sampling over eight original-workload processes per image, not precise
elapsed times. Reports omit each function with fewer than five self samples.

| Case | Wall / retired instructions | Self samples, base → candidate | Classification |
| --- | --- | --- | --- |
| 3d-raytrace | 1.253 / 1.104 | wide activation 50 → 95; decoded static 5,372 → 4,937 | Mixed additional dynamic instructions and time per instruction. Wide dispatch dominates the visible samples, but the short workload is too sparse to locate the extra instructions inside a named function. |
| method_call | 1.226 / 1.003 | typed VM loop 2,212 → 3,011; call_leaf 1,455 → 1,414; new CacheData get_value 1,276 | Layout/dependent-load signature, with essentially flat total instructions. Typed loop grows from 3,024 to 3,434 static instructions; live guarded slot access replaces the old getter. Profiles localize the cost but do not isolate code placement from dependent loads. |
| hash-map | 1.143 / .980 | wide activation 2,158 → 2,460; new get_value 535 | Time-per-instruction signature despite fewer instructions and smaller wide dispatcher. New live slot access is visible; a hardware cache-miss cause is unqualified. |
| gaussian-blur | 1.121 / 1.103 | typed-array bundled kernel 928 → 923 and numeric leaf 604 → 610 | Additional whole-case instructions outside the unchanged dominant kernels (9,155 and 1,548 decoded static instructions). VM/property dispatch changes are candidates, but no per-function dynamic attribution establishes the extra-instruction source. |
| array_index_of | 1.106 / .968 | typed Wide loop 836 → 1,077; new first_prototype_data 397 and load_data 270 | Time-per-instruction/dependent prototype-load signature with fewer total instructions. Typed loop grows from 3,006 to 3,443 static instructions. Layout versus memory dependency remains unresolved. |

No dominant allocator appears in the retained self tables; this does not prove
zero allocations. Hardware cycles and cache misses are unavailable, so wall per
retired instruction is only a proxy, not measured CPI. Per-function dynamic
retired instructions are also unavailable; static counts and sampling cannot
be multiplied into them. All exact-image outputs agree. Whole-case unprofiled
instruction medians are recorded separately in the summary.

A same-input raytrace quality supplement
`20261031T015510Z-t077-s2-r3-raytrace-sample-quality` failed at base repetition 9
because no function met sample's five-self-sample threshold. It yielded no
complete paired result and is not used to replace the original sparse pair.
No workload scaling, address search or runtime experiment followed.

The pass does not causally establish a removable structural design cost.
Therefore no speculative runtime fix is applied. Candidate is frozen at
2cccf17d (runtime a8e7709f plus regenerated x86 order); subsequent diagnostic
and evidence commits preserve those inputs. The typed own-dense-write proposal
at 6e8f7d99 remains unimplemented. External payoff is unresolved. Final
qualification follows; formal, held-out and RSS acceptance remain lead-owned.
