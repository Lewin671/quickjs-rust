# S2 array header target (queued after construction repair)

Freeze this architecture before runtime implementation. Accepted main
`dcee3c56` has a 56-byte hot array header; S2 has 88 bytes plus optional
32-byte holes state and a separately boxed materialized NamedStore.
The two retained-array scripts are recorded in the round-1 measurement receipt.

Target on both macOS architectures:

- `elements: RefCell<Vec<Value>>` 32 bytes.
- `state: Cell<u64>` 8 bytes: logical length in the low 60 bits; four flags
  (extensible, length writable, sealed, frozen) above it. All JS lengths fit;
  internal host overflows must never silently truncate.
- `seed: Cell<Option<Rc<NamedStorage>>>` 8 bytes, holding only an empty authority.
- `cold: OnceCell<Box<ArrayColdData>>` 8 bytes. Total hot payload **56 bytes**.
- Cold data owns `RefCell<NamedStore>` and `RefCell<BTreeSet<usize>>`; no second
  named table or payload index. Expected cold payload 104 bytes, asserted live.

The realm owns one immutable empty array seed with explicit intrinsic prototype
and atom domain. Normal arrays clone that seed once; creation performs no root
hashing, per-array named payload allocation, or individual layout clone. A seed
requested during bootstrap before the intrinsic exists stays uncached.

First named/holes/prototype mutation clones the empty authority into cold data
and takes/drops the array's seed owner, so prototype replacement cannot pin a
retired edge. Cold references remain stable through OnceCell. Read-only probes
never materialize it. Pure seed reads move the Cell owner into a scoped guard
and restore it on Drop, without Rc traffic, allocations or any JS callback.
Live array prototypes use cold prototype-mode identity and existing generations.

Migrate constructors/realm seed first, packed length/flag accessors second,
then named/prototype/teardown consumers. Preserve every observable algorithm.
Test sparse length, holes, descriptors, symbols, prototype mutation/identity,
realm bootstrap and pure-probe allocation; assert sizes on arm64/x86_64.
Re-run the same 300k-array called/no-call RSS scripts versus main and round-1,
then the full required correctness/counter/triage handoff gates. No unsafe,
new dependency, cached Value, benchmark recognizer or held-out input.
