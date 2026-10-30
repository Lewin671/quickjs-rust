# S2 construction follow-up after sequence triage

Frozen before the next construction repair. Base remains `dcee3c56`.
Job `20261030T183538Z-t077-s2-r2-sequences-triage-r2`, candidate `e320817e`,
shows binary-trees instructions .9314 and JSON financial .9182, but wall
1.0488/1.0584; graphlib 1.1504/1.0979, class fields 1.2921/1.1836,
cdjs 1.1549/1.0627, string_slice 1.1143/1.1013 and top-level calls
1.0567/1.0756. It is labelled likely loss, never an acceptance result.
The login-shell job ending 183324Z failed host/recipe identity before measuring.
The direct class initializer entry was subsequently repaired at `3c1c377a`;
`b488bff8` explicitly preserves typed-loop overwrite-only stores.

The before count for graphlib has 239,939 transition misses versus 63,902 hits
and 23.905 whole-program allocator requests/object versus base 17.569.
Its dynamic adjacency maps repeatedly add distinct keys. Broad top-level
calls add four million storage reads for two million global stores: version
check, descriptor reprobe and publication borrow the same authority separately.

Next general mechanisms, preserving the sole named authority:

1. Fuse prepared writable-global slot guards and publication into one exclusive
   borrow. Keep realm/local cell publication and every fallback intact.
2. Cache the empty receiver layout in constructor cold metadata, guarded by
   actual prototype identity/flavor and atom domain. The cache retains only
   schema metadata and weak prototype identities, never an old JS prototype.
   It skips owner/root hashing; every new receiver still owns its actual edge.
3. Reuse JsString leases for computed writes and consume one key lookup for
   existing data or creation. For dynamic additions at a layout with more than
   eight live outgoing transitions, use its private dictionary instead of
   creating another branch. Stable named/class/literal/JSON recipes retain
   shared-layout construction. No benchmark/source/bytecode template admission.
4. Recollect counts and the same triage cases. Resolve broad primitive reads
   from measured evidence; do not cache Values or weaken live-slot validation.

No new dependency, unsafe, binding snapshot, future-field publication, second
property index, or held-out work. Array header work remains next after the
construction target is met. Only non-timing jobs in iMac local 02:45–05:00.
