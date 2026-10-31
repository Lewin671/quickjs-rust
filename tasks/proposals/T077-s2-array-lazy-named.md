# S2 hole-only array metadata and owned dense writes

Freeze before implementation at e0c16e96. Public86 still loses: development
.956719 wall, external 1.027208, selection 1.004109 versus dcee3c56. Paired
job `20261031T002951Z-t077-s2-r2-cache-owner-counts` measures only three seed
reads for nsieve, but 43,201 for 3d-morph and 22,614 for 3d-raytrace. Do not
attribute nsieve's loss to per-element seed reads. Its public input initializes
indices starting at 2, leaving holes at 0/1 throughout the hot loop.

Source shows hole creation eagerly allocates ArrayColdData's full NamedStore.
Subsequent indexed probes borrow it merely to discover it is empty. Separate
hole cold data from a second, lazy named payload: keep the 56-byte hot header;
ArrayColdData becomes holes plus OnceCell<Box<RefCell<NamedStore>>>. Hole-only
arrays retain their actual default seed. Only named/prototype/prototype-mode
mutation forks the seed into the sole named authority and releases the seed.
Pure queries must not initialize either payload. Preserve iterative teardown.

Additionally, interpreter/compact index assignment checks prototypes even when
an existing own dense writable element completely determines [[Set]]. Add a
common ArrayRef overwrite API: verify live logical bounds, frozen state, the
target hole and any own indexed descriptor; then overwrite the present dense
value under one mutable borrow. Prototype metadata is irrelevant on this hit.
Holes, growth, indexed descriptors and failures retain the existing full path.
This is descriptor/type admission, not a source or bytecode template; unrelated
holes do not forbid overwriting an owned data element.

Verify holes/default seed/named initialization independently, prototype changes,
custom/proxy/inherited readonly or setter cases, own accessors, sealed/frozen
arrays, assignment results and iteration/key ordering. Header remains 56 on
both targets; report the smaller hole payload and separate named allocation.
No new dependency, unsafe, native/S3 entry or binding API change, no held-out
run. Run array/descriptor fixtures, verified tiers, full gates, work and public
triage; complete exact scans bind final retained source and correct CLI agents.
