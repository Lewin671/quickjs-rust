# T056 stage A: allocation sizes and native event inventory

- Status: complete baseline/8f inventory; follow-up verification is in the linked diagnosis.
- Implementation: no allocator instrumentation in production; native macOS event logging.
- Verified at: f58c696e / b0dfc65d and8f7d97c7 / b0b84489, x86_64 agents release;
  jobs051608Z (types),051737Z (events),053120Z (host classes), all20261021/done0/no DISTURBED.
- Evidence: [producers and interpretation](../diagnostics/t056-allocation-probe/README.md),
  [fixes and new-image checks](T056-m1-stageA-allocation-diagnosis.md).
- Unresolved: allocation counts do not price latency; new image wall remains lead work.
- Next action: use new-image receipts in the diagnosis; do not repeat the historical probe for timing.

## Named heap layouts

Complete compiler inventories contain15341 base types and a separate candidate inventory;
closure/generic compiler types are not confused with independent heap allocations. The table
covers changed named runtime heap owners and changed inline/vector elements, plus newly
introduced layout allocations. Sizes include enum/niche/alignment padding. RefCell's borrow
word and Rc's two counters must be included before asking malloc for a size class.
`classes.sh` measured both good_size and actual usable_size for requests1..4096 on macOS12.6
(21G115); they agree for every request named below. This is host-specific evidence.

| Allocation or containing element | base ->8f bytes requested | usable class (host) / ownership |
| --- | --- | --- |
| Rc<ObjectData> |128 ->136 (payload112 ->120)|128 ->144; follow-up restores payload112/request128 |
| Rc<RealmState> |1024 ->1032 (payload1008 ->1016)|1024 ->1536;24 in unpack, mostly startup helpers |
| Rc<RefCell<NamedPropertyCacheState>> |264 ->296 (state240 ->272)|272 ->304;32 in unpack |
| Box<FunctionColdState> |152 ->128|160 ->128 |
| Box<ArrayColdData> |144 ->136|144 ->144; no class crossing |
| Rc<ObjectLiteralShape> |88 ->80 (payload72 ->64)|96 ->80 |
| Rc<AtomDomain::DomainState> |80 ->192 (payload64 ->176)|80 ->192 |
| ObjectData.properties: RefCell<PropertyStorage> |48 ->56 (store40 ->48)|inline, not a second allocation |
| NamedPropertyCacheEntry |48 ->56|inline4-entry array in the cache state above |
| SharedSlot |new56|inline guard in cache entries/ShapeWays, not an independent request |
| ShapeWays |48 ->80|Vec element: n*48 ->n*80; e.g. n=4:192 ->320 |
| NamedWriteSite |64 ->80|Vec element: n*64 ->n*80; e.g. n=4:256 ->320 |
| CreationCache / CreationProof |32/24 ->48/40|inline in write sites; no independent request |
| InlineCallee |16 ->8|Vec element; e.g. n=4:64 ->32 |
| OrderedDataPropertyBuilder |48 ->56|stack builder, not an additional heap box |
| Small property entry |48 (name+Property) ->32 (Property)|Vec element shrinks; keys move to layout |
| Box<DynamicPropertyStorage> |80 ->removed|old class80; layout/dictionary is the replacement authority |
| Rc<LayoutNode> |new160 (Shape/Dictionary136, enum144)|class160; shared shapes or private dictionary |
| Rc<DomainRoots> |new48 (payload32)|class48 |
| dictionary identity Rc<()> |new16|class16; independent identity for weak slot caches |
| Field |new32|boxed slice element; n*32 |
| AppendEdge / EditEdge |new40/16|Vec elements; initial n=4 requests160/64 |
| Box<shape text index> |new32 plus hash-table buckets|class32; bucket requests by field capacity |
| ContentCache |new40|inline domain state, buckets separately allocated |
| Rc<AtomEntry> |48 ->48|class48 unchanged; newly reached by dynamic keys in stage A |
| Rc<StringData> |88 ->88 (payload72 ->72)|class96 unchanged; JsString handle remains8 |
| string bytes / regexp character buffers |same element sizes|dynamic length/capacity, see event counts |
| scalar register backing |4096 ->4096 bytes|production alignment4096, pooled; not nano-sized |

Bytecode and Function's main allocation layout are unchanged; cold named-table state changes
are accounted for above. T063 is already in the frozen base, not a new string-layout change.
Public diagnostic Counters also grows304 ->400, but is not a production heap allocation in
these canonical builds. No per-call name-keyed binding snapshot is introduced or proposed.

## Complete allocation events

`20261021T051737Z-t056-alloc-native-counts` uses original canonical images. ALLOC events
include freed blocks and realloc's allocation side; VM reservations are not mixed into malloc
counts. These are diagnostic process totals, not peak/live bytes or timing-harness counters.

| Wrapper/workload | base events / requested bytes |8f events / requested bytes |
| --- | ---: | ---: |
| empty |7653 /819359|8728 /1303905 |
| whole unpack |821494 /52708236|835335 /56449865 |
| string_slice100000 |320296 /16157622|321525 /16793744 |

Unpack adds13841 events (+1.685%) and3741629 requested bytes; subtracting the matched empty
control leaves12766 extra events and3257083 extra bytes. This does not establish a wall cause.
For string_slice the event delta1229 is mostly the empty delta1075; it does not show a hot
string-size-class jump. Stack logging and the eval wrapper perturb the process environment.

|16-byte request class, unpack | base count |8f count |
| --- | ---: | ---: |
|16|582296|581985|
|32|29869|35268|
|48|3376|8989|
|64|81899|81963|
|80|176|136|
|96|118655|118669|
|112|355|355|
|128|910|430|
|144|255|341|
|160|423|2712|
|176|314|314|
|192|359|246|
|208|5|5|
|224|51|78|
|240|19|19|
|256|140|138|
|request >256 (not rounded-class bytes)|2392|3687|

The dominant regexp/string requests are stable:81490 requests of88 from replace results,
81474 of64 for its working buffer,16188 of88 from fast-native strings,10534 of88 from
primitive concatenation,9806 of88 from split results, plus128 cached ASCII StringData.
The 136-byte request occurs only110 times (78 in empty), so ObjectData's crossing alone
cannot explain the allocator-heavy profile. The size-class theory is narrowed, not confirmed.

Concrete added work:4868 requests each of24 and48 in missing-key atom interning;224
3216-byte index buckets in `write_existing_own_data_property -> Shape::first_lookups`.
These are intermediate growing layouts, not final dictionaries. The follow-up prevents a
missing write from creating that index. Remaining per-key AtomEntry costs are inherent in
the current authority/identity design and are not removed by a name-keyed binding snapshot.
Native generic RawVec symbols can alias under LTO (e.g. an edge vector named DisposeResource);
semantic callers plus size identify the path, not the last mangled symbol alone.

## Follow-up native event check

`20261021T054028Z-t056-compact-native-counts` is done0/no DISTURBED,71s,
using pinned canonical-producer image
`3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f`.
All three source hashes match the prior wrappers. No Rust instrumentation or timing.

| Wrapper/workload | follow-up events / requested bytes |
| --- | ---: |
| empty |8726 /1299979|
| unpack |834407 /55073123|
| string_slice100000 |321525 /16793042|

Unpack removes928 events and1376742 requested bytes vs8f. Shape index paths go
from469 indexes/938 allocation events/1385712 bytes to5/10/9840: **464 transient
indexes avoided**,928 events and1375872 bytes removed on that path. The largest
3216-byte bucket falls226 ->2 on the index path (all request3216 events227 ->3).
The main string allocation counts remain stable. Of110 old136-byte requests,
102 move to128; the remaining8 are other types. String_slice event count stays
the same: do not claim its allocation-heavy loop is repaired.

Raw unpack gzip SHA: old
`12b87119085ef0495a7a253c1dff25fd263f9cae54a20f01d83cce47112ee361`, new
`c21a1ef1e2b55b712a8230945c40590070c29f7f67b9ffe0501e553f33a13597`.
Old/new summary SHA:
`52d5b3db5b515c8460e9d628abdaf5eac1262c4c84c314fe37f778d98b796203` /
`8af83a1e7fb9797b9ad9e7baab05d8985b72e8990eacf8a603113e66559e51d3`.
Counts and bytes establish removed work, not a measured wall improvement.
