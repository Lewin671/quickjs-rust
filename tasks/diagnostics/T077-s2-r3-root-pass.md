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
