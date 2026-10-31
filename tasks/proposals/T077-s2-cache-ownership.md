# S2 cache state ownership and immutable site configuration

Freeze before implementation at e1bad6bd. Public86 job
`20261030T235607Z-t077-s2-r2-array-word-public86` still loses: development
.967463 wall, external 1.029473, selection 1.008135. Seven-case instructions
leave nbody 1.11540, nsieve 1.09913, gaussian-blur 1.10256; the encoding change
did not repair their work. Main's typed numeric slot metadata is inline in its
per-program ShapeWays vector; S2 instead puts an Rc cache handle in that vector.

The 01ee5ee6 paired work records show nbody only 6,703 mutable IC metadata reads
but over one million storage reads: mutable metadata alone is not the remaining
own-slot cost. A successful local-site hit still loads an unnecessary Rc handle
before its guard. Gaussian has 1,800,198 IC metadata reads. Source additionally
puts immutable `local_slot` configuration inside RefCell<State>, even though it
is set only by the site constructor and survives clear unchanged.

Move the existing IC methods onto CacheData. Shared interpreter/compact caches
keep NamedPropertyCache(Rc<CacheData>) and delegate through Deref. The typed
loop's already-owned, exclusively borrowed site vector stores CacheData inline.
This is one implementation and authority, with the same mono/four-way/mega
admissions, weak guards and live-slot APIs; no second lookup engine or value
cache. There is no runtime owner-mode branch or new allocation for local sites.

Keep immutable local_slot in CacheData outside mutable State. Shared handles
retain their existing clone/warm behavior. Cloning a compiled program starts
fresh local optimization state while preserving that immutable slot metadata;
cache warmness has no language semantics. Default/clone metadata must remain
value-free and must not pin receivers. Watch code-clone and short-loop costs.
Ordinary shared handle remains eight bytes; report inline site size and counts.

Test owned/shared read-write parity, cold clones, local-slot clear/clone,
prototype invalidation and weak lifetimes. Run focused tiers/verified caches,
strict gates, paired public work and all86 wall triage. No dependency, unsafe,
source/opcode-template matcher, native/S3 entry or binding API change. No formal
or held-out run. If this does not pay, preserve evidence and revert the runtime
experiment rather than claiming an enabling-stage waiver.
