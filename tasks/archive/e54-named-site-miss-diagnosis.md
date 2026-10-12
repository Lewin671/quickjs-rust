# e54 named-read and method-site diagnosis, 2026-10-12

Base e54e33eb9371c5cb4627bd8e590a648085a7fa26. No production runtime changes
are part of this diagnostic. B remains held at 644e1de4/e22f8942; no second attempt.

## Conclusion

The all-cold method calibration was misapplied to corpus method frequencies.
Hash-map's method reads are already hot: 2069997/2070029 method-site reads hit.
Its 3510015 OwnSlot full-probe answers are the primary cache gap, with 720037
primitive/array answers and 179998 direct getters as separate costs. Two-level
inheritance is working: 359992 hot grandprototype answers, no steady-state chain
misses. The 42.1% method-premium proxy from T036 is not a corpus miss-weighted share.

Trees' two recursive itemCheck sites each execute 20632 times, with zero hot hits,
one semantic/ordered/shared-name layout and 20632 distinct receivers. The correct
holder is already cached; the single receiver-identity proof is the blocker.

CDjs has 596719 primitive-string localeCompare reads, all ineligible for the object
hot path, all calling a native fallback; this is a different mechanism from an
alternating-object prototype miss. Its object prototype misses are 395710 receiver
identity failures plus 39584 absent proofs, chiefly VM reads. Shapes are stable:
_treeInsert and successor have one shape; three compareTo sites have two genuinely
different shapes (_value vs x,y), not insertion-order variants. Just 108 misses have
a different hot holder; 435291 misses already retain a valid holder in the cache.
No receiver-layout, holder-layout or lifetime failures were observed in the three
corpora. No prototype-read site clears its cache. Read-site cache updates below
show one/two holder installs, not repeated eviction. The 36.4%/33.3% method proxies
must likewise be replaced by the miss-weighted prices below.

## General fixes and bounds

- A general negative-own-property cache keyed by a shared, descriptor-complete
  layout identity, with live receiver-prototype and holder-layout guards, would
  remove the per-receiver own-miss proof. Constructor identity, numeric epoch or
  property count alone cannot prove absence: own shadows/accessors/proxies and
  delete/reinsert cases must decline. Four receiver identities would not cover
  the unique-node case. No shared negative-layout implementation is built here.
- Cold-to-hot object prototype resolution removes about 320 instructions in the
  three-field calibration. Applied only to identity failures: hash-map 0;
  CDjs 395710*320=126.63M instructions (6.72% of its 1.88445B gap);
  trees 41260*320=13.20M (20.51% of its 64.382M gap). Six-field price gives338
  instructions; CDjs's supported layout-dependent interval is126.6–133.7M.
- The VM never remembers receiver_miss in try_cached_get_string. Remember the
  proven own miss before returning a prototype answer; a stable VM receiver
  then becomes hot. There are 39584 such absent-proof events in CDjs and 806 in
  trees, with 0 in hash-map beyond warmup. Those are upper reach, not guaranteed
  hits: a changing receiver still needs shared-layout proof. VM-exclusive price
  was not measured, so no absolute VM instruction saving is claimed.
- Primitive-string prototype data caching on the hot path needs the effective
  realm/prototype and revision guards plus length/index/exotic semantics. The
  measured lookup increment is 216.5 R vs 120.0 NG instructions; its excess is 96.4,
  not the338.5 object proxy. For 596719 localeCompare reads that excess accounts
  for 57.54M instructions (3.05% of CDjs's gap). No implementation/removal price
  is established; native comparison and native dispatch remain separate.
- These lookup fixes do not remove the plain-call activation envelope: the old
  plain/method 4/8-call calibration has equal native-probe counts, and lookup
  is outside the callee's bank setup/cleanup. CDjs's 596719 localeCompare calls
  take native fallback, so their extra bridge/dispatch costs are not the wide
  activation envelope. Do not charge all native calls to a frame-ABI migration.
- No demonstrated combination brings all three targets to <=1.0 against NG.

## Protocol and cost table

Execution counters observe all wide GetPropNamed/GetPropThis and VM named-read
opcodes, with nested scopes and cache identity retained against address reuse.
Zero unscoped hot probes in all three corpora. Cache probes/updates/clears,
receiver key+descriptor orders, order-independent layouts, shared name pointer
layouts, literal layouts, epochs and prototype depth are recorded. Object names
are metadata only; no name-keyed binding snapshot is added. String wrappers are
not enumerated, avoiding lazy-index materialization. Checksums/stdout match the
sealed official bundle hashes. These counter builds are never timed.

Resolved-call sites in wide and VM are counted; wide admission outcomes distinguish
wide/numeric/fallback. VM direct/generic is grouped; guarded unary Math is outside
this resolved-call census. Named-read totals and zero unscoped probes are the
coverage checks; they are not a claim to count every hoisted accelerator guard as
an ordinary named-read opcode. Site labels use bytecode ordinal, parameter list,
code length, pc and key; anonymous/root names are not source-function identity.

Five AB/BA pairs, N=200000, median paired (T8–T4)/(4N). All bodies are nonempty;
matched function/number/object/array move controls are also 4/8 increments. Counters
at N=32 confirm wide admission and the predicted answer family in both forms.
Prices below subtract the matched move/use packet. They include wide read/guard,
lookup and transport; they are not exclusive static helper instruction counts.
Mixed result families are calibrated per site; unpriced answers remain explicit.
Do not add this table as an exclusive accounting of the whole process gap.

| Fallback-inclusive named-read increment | R instructions | NG instructions | Old A instructions |
| --- | ---: | ---: | ---: |
| exact | 185.028 | 63.797 | 222.961 |
| literal | 215.708 | 64.009 | 253.875 |
| shared | 90.017 | 63.901 | 89.413 |
| own_number | 245.974 | 64.011 | 100.952 |
| own_function | 207.351 | 51.164 | 87.597 |
| proto_hit | 117.617 | 68.999 | 123.614 |
| proto_miss3 | 437.526 | 69.083 | 443.709 |
| proto_miss6 | 455.515 | 60.859 | 461.485 |
| proto_miss12 | 491.433 | 60.915 | 497.709 |
| grand_hit | 160.392 | 79.001 | 163.532 |
| grand_miss6 | 499.436 | 71.005 | 507.567 |
| function_prototype | 2098.179 | 69.023 | 2100.748 |
| string_proto | 216.479 | 120.046 | 217.453 |
| array_length | 96.009 | 58.933 | 96.922 |
| getter | 1065.232 | 477.066 | 1068.730 |
| own_object | 232.277 | 50.861 | 87.575 |
| own_array | 207.686 | 50.954 | 87.086 |

## All sites, ranked by calibrated excess instructions

Each row records execution/hit/miss/method counts, fallback answer counts and
R/NG instruction prices, plus shape/proof evidence. Prices are estimates from
the matched packet; they are not instruction tracing of the instrumented build.

### access-binary-trees

Totals: {"reads": 167485, "hits": 125393, "misses": 42092, "method_site_reads": 42076, "method_site_hits": 0, "method_site_misses": 42076}.

| Site | Executions / hot / miss / method | Priced excess instructions | Fallback counts @ R/NG instructions | Reasons; guards | Ordered / unordered / shared-name / receivers; depth; cache events |
| --- | --- | ---: | --- | --- | --- |
| B3:root()/ops29/pc8/itemCheck | 20632/0/20632/20632 | 7601726 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:20631@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 20630, 'receiver-proof-absent': 1}; {'hot-holder-matches': 20631, 'receiver-proof-invalid': 20631, 'valid-holder-already-cached': 20631} | 1/1/1/20632; depth={'1': 20632}; events={'update_from_prototype': 1} |
| B3:anonymous()/ops29/pc12/itemCheck | 20632/0/20632/20632 | 7601726 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:20631@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 20630, 'receiver-proof-absent': 1}; {'hot-holder-matches': 20631, 'receiver-proof-invalid': 20631, 'valid-holder-already-cached': 20631} | 1/1/1/20632; depth={'1': 20632}; events={'update_from_prototype': 1} |
| B3:root()/ops29/pc0/left | 42072/42069/3/0 | 1098692 | hot-SharedSlot:42069@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/42072; depth={'0': 42072}; events={'update': 2} |
| B3:anonymous()/ops29/pc3/item | 21440/21437/3/0 | 559858 | hot-SharedSlot:21437@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/21440; depth={'0': 21440}; events={'update': 2} |
| B3:root()/ops29/pc6/item | 20632/20629/3/0 | 538756 | hot-SharedSlot:20629@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/20632; depth={'0': 20632}; events={'update': 2} |
| B3:root()/ops29/pc7/left | 20632/20629/3/0 | 538756 | hot-SharedSlot:20629@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/20632; depth={'0': 20632}; events={'update': 2} |
| B3:anonymous()/ops29/pc11/right | 20632/20629/3/0 | 538756 | hot-SharedSlot:20629@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/20632; depth={'0': 20632}; events={'update': 2} |
| B1:vm()/ops254/pc142/itemCheck | 400/0/400/400 | 147377 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:399@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 399}; {'hot-holder-matches': 399, 'receiver-proof-invalid': 399, 'valid-holder-already-cached': 399} | 1/1/1/400; depth={'1': 400}; events={'update_from_prototype': 1} |
| B1:vm()/ops254/pc157/itemCheck | 400/0/400/400 | 147377 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:399@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 399}; {'hot-holder-matches': 399, 'receiver-proof-invalid': 399, 'valid-holder-already-cached': 399} | 1/1/1/400; depth={'1': 400}; events={'update_from_prototype': 1} |
| B1:vm()/ops254/pc82/itemCheck | 4/0/4/4 | 1474 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:3@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 3}; {'hot-holder-matches': 3, 'receiver-proof-invalid': 3, 'valid-holder-already-cached': 3} | 1/1/1/4; depth={'1': 4}; events={'update_from_prototype': 1} |
| B1:vm()/ops254/pc196/itemCheck | 4/0/4/4 | 1474 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:3@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 3}; {'hot-holder-matches': 3, 'receiver-proof-invalid': 3, 'valid-holder-already-cached': 3} | 1/1/1/4; depth={'1': 4}; events={'update_from_prototype': 1} |
| B1:vm()/ops254/pc62/max | 4/0/4/4 | 364 | own-lookup:1@unpriced; probe-own-Exact:3@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/1; depth={'0': 4}; events={'update': 1} |
| B1:vm()/ops254/pc33/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |

#### Resolved-call sites and admission

| Site | Executions, callee family, admission counts |
| --- | --- |
| B1:pc143:vm:method | {'executions': 400, 'function:anonymous:ordinary:unbound:bytecode': 400, 'vm-direct-or-generic': 400} |
| B1:pc158:vm:method | {'executions': 400, 'function:anonymous:ordinary:unbound:bytecode': 400, 'vm-direct-or-generic': 400} |
| B1:pc197:vm:method | {'executions': 4, 'function:anonymous:ordinary:unbound:bytecode': 4, 'vm-direct-or-generic': 4} |
| B1:pc67:vm:method | {'executions': 4, 'function:max:ordinary:unbound:MathMax': 4, 'vm-direct-or-generic': 4} |
| B1:pc83:vm:method | {'executions': 4, 'function:anonymous:ordinary:unbound:bytecode': 4, 'vm-direct-or-generic': 4} |
| B2:pc10:wide:plain | {'executions': 20632, 'function:bottomUpTree:ordinary:unbound:bytecode': 20632, 'wide': 20632} |
| B2:pc16:wide:plain | {'executions': 20632, 'function:bottomUpTree:ordinary:unbound:bytecode': 20632, 'wide': 20632} |
| B3:pc13:wide:method | {'executions': 20632, 'function:anonymous:ordinary:unbound:bytecode': 20632, 'wide': 20632} |
| B3:pc9:wide:method | {'executions': 20632, 'function:anonymous:ordinary:unbound:bytecode': 20632, 'wide': 20632} |
### hash-map

Totals: {"reads": 8100143, "hits": 3599982, "misses": 4500161, "method_site_reads": 2070029, "method_site_hits": 2069997, "method_site_misses": 32}.

| Site | Executions / hot / miss / method | Priced excess instructions | Fallback counts @ R/NG instructions | Reasons; guards | Ordered / unordered / shared-name / receivers; depth; cache events |
| --- | --- | ---: | --- | --- | --- |
| B7:_findKeyEntry(key,index,keyHash)/ops32/pc0/_elementData | 540000/0/540000/0 | 84635305 | own-lookup:1@unpriced; probe-own-OwnSlot:539999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 539999}; {} | 1/1/1/1; depth={'0': 540000}; events={'update': 1} |
| B11:_getEntry(key)/ops23/pc4/_elementData | 450000/0/450000/0 | 70529395 | own-lookup:1@unpriced; probe-own-OwnSlot:449999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 449999}; {} | 1/1/1/1; depth={'0': 450000}; events={'update': 1} |
| B5:run()/ops180/pc68/value | 90000/0/90000/0 | 52934394 | direct-getter:89999@1065.2/477.1i; general-get:1@unpriced | {'empty-hot-entry': 90000}; {} | 1/1/1/90000; depth={'1': 90000}; events={} |
| B5:run()/ops180/pc66/key | 90000/0/90000/0 | 52934394 | direct-getter:89999@1065.2/477.1i; general-get:1@unpriced | {'empty-hot-entry': 90000}; {} | 1/1/1/90000; depth={'1': 90000}; events={} |
| B15:hasNext()/ops57/pc0/_futureEntry | 180001/0/180001/0 | 32753115 | own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-OwnSlot:179998@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3, 'unsupported-hot-OwnSlot': 179997}; {} | 1/1/1/1; depth={'0': 180001}; events={'update': 2} |
| B15:hasNext()/ops57/pc9/_position | 180001/0/180001/0 | 32753115 | own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-OwnSlot:179998@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3, 'unsupported-hot-OwnSlot': 179997}; {} | 1/1/1/1; depth={'0': 180001}; events={'update': 2} |
| B15:hasNext()/ops57/pc16/_position | 180001/0/180001/0 | 32753115 | own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-OwnSlot:179998@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3, 'unsupported-hot-OwnSlot': 179997}; {} | 1/1/1/1; depth={'0': 180001}; events={'update': 2} |
| B15:hasNext()/ops57/pc10/_associatedMap | 180001/0/180001/0 | 32654605 | own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-OwnSlot:179998@232.3/50.9i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3, 'unsupported-hot-OwnSlot': 179997}; {} | 1/1/1/1; depth={'0': 180001}; events={'update': 2} |
| B15:hasNext()/ops57/pc14/_associatedMap | 180001/0/180001/0 | 32654605 | own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-OwnSlot:179998@232.3/50.9i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3, 'unsupported-hot-OwnSlot': 179997}; {} | 1/1/1/1; depth={'0': 180001}; events={'update': 2} |
| B15:hasNext()/ops57/pc11/_elementData | 180001/0/180001/0 | 28211821 | own-lookup:1@unpriced; probe-own-OwnSlot:180000@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 180000}; {} | 1/1/1/1; depth={'0': 180001}; events={'update': 1} |
| B15:hasNext()/ops57/pc15/_elementData | 180001/0/180001/0 | 28211821 | own-lookup:1@unpriced; probe-own-OwnSlot:180000@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 180000}; {} | 1/1/1/1; depth={'0': 180001}; events={'update': 1} |
| B10:get(key)/ops19/pc1/_getEntry | 450000/449998/2/450000 | 21878753 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:449998@117.6/69.0i; prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 450000}; events={'update_from_prototype': 1} |
| B11:_getEntry(key)/ops23/pc10/_findKeyEntry | 450000/449998/2/450000 | 21878753 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:449998@117.6/69.0i; prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 450000}; events={'update_from_prototype': 1} |
| B5:run()/ops180/pc30/get | 449991/449989/2/449991 | 21878315 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:449989@117.6/69.0i; prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 449991}; events={'update_from_prototype': 1} |
| B11:_getEntry(key)/ops23/pc5/length | 450000/0/450000/0 | 16684271 | primitive-prototype:450000@96.0/58.9i | {'eligibility-no-hot-probe': 450000}; {} | 0/0/0/0; depth={}; events={} |
| B17:_makeNext()/ops83/pc22/_position | 90000/0/90000/0 | 16376497 | own-lookup:1@unpriced; probe-own-OwnSlot:89999@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 89999}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 1} |
| B6:put(key,value)/ops86/pc20/_modCount | 90000/0/90000/0 | 16376497 | own-lookup:1@unpriced; probe-own-OwnSlot:89999@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 89999}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 1} |
| B6:put(key,value)/ops86/pc39/_elementCount | 90000/0/90000/0 | 16376497 | own-lookup:1@unpriced; probe-own-OwnSlot:89999@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 89999}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 1} |
| B18:_checkConcurrentMod()/ops18/pc0/_expectedModCount | 90000/0/90000/0 | 16376315 | own-lookup:2@unpriced; probe-own-OwnSlot:89998@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2, 'unsupported-hot-OwnSlot': 89997}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 2} |
| B17:_makeNext()/ops83/pc15/_futureEntry | 90000/0/90000/0 | 16376315 | own-lookup:2@unpriced; probe-own-OwnSlot:89998@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2, 'unsupported-hot-OwnSlot': 89997}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 2} |
| B6:put(key,value)/ops86/pc48/_threshold | 90000/0/90000/0 | 16376315 | own-lookup:2@unpriced; probe-own-OwnSlot:89998@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2, 'unsupported-hot-OwnSlot': 89997}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 2} |
| B18:_checkConcurrentMod()/ops18/pc1/_associatedMap | 90000/0/90000/0 | 16327061 | own-lookup:2@unpriced; probe-own-OwnSlot:89998@232.3/50.9i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2, 'unsupported-hot-OwnSlot': 89997}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 2} |
| B17:_makeNext()/ops83/pc19/_associatedMap | 90000/0/90000/0 | 16327061 | own-lookup:2@unpriced; probe-own-OwnSlot:89998@232.3/50.9i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2, 'unsupported-hot-OwnSlot': 89997}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 2} |
| B17:_makeNext()/ops83/pc34/_currentEntry | 90000/0/90000/0 | 16327061 | own-lookup:2@unpriced; probe-own-OwnSlot:89998@232.3/50.9i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2, 'unsupported-hot-OwnSlot': 89997}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 2} |
| B16:next()/ops10/pc3/_currentEntry | 90000/0/90000/0 | 16327061 | own-lookup:2@unpriced; probe-own-OwnSlot:89998@232.3/50.9i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2, 'unsupported-hot-OwnSlot': 89997}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 2} |
| B17:_makeNext()/ops83/pc20/_elementData | 90000/0/90000/0 | 14105754 | own-lookup:1@unpriced; probe-own-OwnSlot:89999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 89999}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 1} |
| B8:_createHashedEntry(key,index,hash)/ops28/pc6/_elementData | 90000/0/90000/0 | 14105754 | own-lookup:1@unpriced; probe-own-OwnSlot:89999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 89999}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 1} |
| B6:put(key,value)/ops86/pc4/_elementData | 90000/0/90000/0 | 14105754 | own-lookup:1@unpriced; probe-own-OwnSlot:89999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 89999}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 1} |
| B8:_createHashedEntry(key,index,hash)/ops28/pc9/_elementData | 90000/0/90000/0 | 14105754 | own-lookup:1@unpriced; probe-own-OwnSlot:89999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 89999}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 1} |
| B10:get(key)/ops19/pc7/_value | 450000/449997/3/0 | 11752314 | hot-SharedSlot:449997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/90000; depth={'0': 450000}; events={'update': 2} |
| B7:_findKeyEntry(key,index,keyHash)/ops32/pc6/_origKeyHash | 450000/449997/3/0 | 11752314 | hot-SharedSlot:449997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/90000; depth={'0': 450000}; events={'update': 2} |
| B7:_findKeyEntry(key,index,keyHash)/ops32/pc11/_key | 450000/449997/3/0 | 11752314 | hot-SharedSlot:449997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/90000; depth={'0': 450000}; events={'update': 2} |
| B18:_checkConcurrentMod()/ops18/pc2/_modCount | 90000/0/90000/0 | 10910659 | own-lookup:1@unpriced; probe-own-Exact:89999@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 89999}; {} | 1/1/1/1; depth={'0': 90000}; events={'update': 1} |
| B5:run()/ops180/pc57/hasNext | 90000/89998/2/90000 | 7325948 | general-prototype-chain:1@499.4/71.0i; hot-GrandprototypeSlot:89998@160.4/79.0i; prototype-own-miss:1@499.4/71.0i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'2': 90000}; events={'update_from_prototype': 1} |
| B17:_makeNext()/ops83/pc1/_checkConcurrentMod | 90000/89998/2/90000 | 7325948 | general-prototype-chain:1@499.4/71.0i; hot-GrandprototypeSlot:89998@160.4/79.0i; prototype-own-miss:1@499.4/71.0i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'2': 90000}; events={'update_from_prototype': 1} |
| B17:_makeNext()/ops83/pc4/hasNext | 90000/89998/2/90000 | 7325948 | general-prototype-chain:1@499.4/71.0i; hot-GrandprototypeSlot:89998@160.4/79.0i; prototype-own-miss:1@499.4/71.0i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'2': 90000}; events={'update_from_prototype': 1} |
| B16:next()/ops10/pc1/_makeNext | 90000/89998/2/90000 | 7325948 | general-prototype-chain:1@499.4/71.0i; hot-GrandprototypeSlot:89998@160.4/79.0i; prototype-own-miss:1@499.4/71.0i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'2': 90000}; events={'update_from_prototype': 1} |
| B15:hasNext()/ops57/pc12/length | 180001/0/180001/0 | 6673745 | primitive-prototype:180001@96.0/58.9i | {'eligibility-no-hot-probe': 180001}; {} | 0/0/0/0; depth={}; events={} |
| B5:run()/ops180/pc62/next | 90000/89997/3/90000 | 4376650 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:89997@117.6/69.0i; prototype-own-miss:1@455.5/60.9i; vm-prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 2}; {'hot-holder-matches': 2, 'receiver-proof-invalid': 2, 'valid-holder-already-cached': 2} | 1/1/1/1; depth={'1': 90000}; events={'update_from_prototype': 1} |
| B6:put(key,value)/ops86/pc32/_createHashedEntry | 90000/89998/2/90000 | 4376304 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:89998@117.6/69.0i; prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 90000}; events={'update_from_prototype': 1} |
| B6:put(key,value)/ops86/pc10/_findKeyEntry | 90000/89998/2/90000 | 4376304 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:89998@117.6/69.0i; prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 90000}; events={'update_from_prototype': 1} |
| B5:run()/ops180/pc8/put | 89999/89997/2/89999 | 4376256 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:89997@117.6/69.0i; prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 89999}; events={'update_from_prototype': 1} |
| B6:put(key,value)/ops86/pc5/length | 90000/0/90000/0 | 3336854 | primitive-prototype:90000@96.0/58.9i | {'eligibility-no-hot-probe': 90000}; {} | 0/0/0/0; depth={}; events={} |
| B6:put(key,value)/ops86/pc61/_value | 90000/89997/3/0 | 2350400 | hot-SharedSlot:89997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/90000; depth={'0': 90000}; events={'update': 2} |
| B17:_makeNext()/ops83/pc35/_next | 90000/89997/3/0 | 2350400 | hot-SharedSlot:89997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/90000; depth={'0': 90000}; events={'update': 2} |
| B4:_computeThreshold()/ops13/pc2/_loadFactor | 18/0/18/0 | 2911 | own-lookup:2@unpriced; probe-own-OwnSlot:16@246.0/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2, 'unsupported-hot-OwnSlot': 15}; {} | 2/2/2/1; depth={'0': 18}; events={'update': 2} |
| B4:_computeThreshold()/ops13/pc0/_elementData | 18/0/18/0 | 2508 | own-lookup:2@unpriced; probe-own-OwnSlot:16@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 17}; {} | 2/2/2/1; depth={'0': 18}; events={'update': 2} |
| B9:_rehash(capacity)/ops119/pc2/_elementData | 17/0/17/0 | 2508 | own-lookup:1@unpriced; probe-own-OwnSlot:16@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 16}; {} | 1/1/1/1; depth={'0': 17}; events={'update': 1} |
| B1:vm()/ops156/pc83/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops156/pc84/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops156/pc91/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops156/pc92/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops156/pc99/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops156/pc100/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B13:EntryIterator(map)/ops9/pc1/call | 1/0/1/1 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B6:put(key,value)/ops86/pc51/_rehash | 17/15/2/17 | 1519 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:15@117.6/69.0i; prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 17}; events={'update_from_prototype': 1} |
| B9:_rehash(capacity)/ops119/pc78/_computeThreshold | 17/15/2/17 | 1519 | general-prototype-chain:1@455.5/60.9i; hot-PrototypeSlot:15@117.6/69.0i; prototype-own-miss:1@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 17}; events={'update_from_prototype': 1} |
| B4:_computeThreshold()/ops13/pc1/length | 18/0/18/0 | 667 | primitive-prototype:18@96.0/58.9i | {'eligibility-no-hot-probe': 18}; {} | 0/0/0/0; depth={}; events={} |
| B9:_rehash(capacity)/ops119/pc3/length | 17/0/17/0 | 630 | primitive-prototype:17@96.0/58.9i | {'eligibility-no-hot-probe': 17}; {} | 0/0/0/0; depth={}; events={} |
| B5:run()/ops180/pc49/entrySet | 1/0/1/1 | 395 | general-prototype-chain:1@455.5/60.9i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'1': 1}; events={'update_from_prototype': 1} |
| B3:HashMap(capacity,loadFactor)/ops73/pc45/_computeThreshold | 1/0/1/1 | 395 | general-prototype-chain:1@455.5/60.9i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'1': 1}; events={'update_from_prototype': 1} |
| B2:vm()/ops154/pc123/validate | 1/0/1/0 | 368 | general-prototype-chain:1@437.5/69.1i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'3': 1}; events={'update_from_prototype': 1} |
| B2:vm()/ops154/pc85/prepareForNextIteration | 1/0/1/0 | 368 | general-prototype-chain:1@437.5/69.1i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'3': 1}; events={'update_from_prototype': 1} |
| B5:run()/ops180/pc52/iterator | 1/0/1/1 | 368 | general-prototype-chain:1@437.5/69.1i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'1': 1}; events={'update_from_prototype': 1} |
| B3:HashMap(capacity,loadFactor)/ops73/pc37/capacity | 1/0/1/0 | 368 | general-prototype-chain:1@437.5/69.1i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'3': 1}; events={'update_from_prototype': 1} |
| B2:vm()/ops154/pc105/runIteration | 1/0/1/1 | 368 | general-prototype-chain:1@437.5/69.1i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'1': 1}; events={'update_from_prototype': 1} |
| B9:_rehash(capacity)/ops119/pc29/length | 1/0/1/0 | 37 | primitive-prototype:1@96.0/58.9i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={} |
| B14:AbstractMapIterator(map)/ops21/pc2/_modCount | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B15:hasNext()/ops57/pc22/_position | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B9:_rehash(capacity)/ops119/pc46/_origKeyHash | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B9:_rehash(capacity)/ops119/pc50/_next | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B12:iterator()/ops8/pc1/_associatedMap | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B9:_rehash(capacity)/ops119/pc28/_elementData | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B9:_rehash(capacity)/ops119/pc31/_elementData | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B9:_rehash(capacity)/ops119/pc34/_elementData | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |

#### Resolved-call sites and admission

| Site | Executions, callee family, admission counts |
| --- | --- |
| B10:pc3:wide:method | {'executions': 450000, 'function:_getEntry:ordinary:unbound:bytecode': 450000, 'wide': 450000} |
| B11:pc14:wide:method | {'executions': 450000, 'function:_findKeyEntry:ordinary:unbound:bytecode': 450000, 'wide': 450000} |
| B11:pc2:wide:plain | {'executions': 450000, 'function:computeHashCode:ordinary:unbound:bytecode': 450000, 'numeric': 450000} |
| B13:pc4:wide:method | {'executions': 1, 'fallback': 1, 'function:call:ordinary:unbound:FunctionPrototypeCall': 1} |
| B16:pc2:wide:method | {'executions': 90000, 'function:_makeNext:ordinary:unbound:bytecode': 90000, 'wide': 90000} |
| B17:pc2:wide:method | {'executions': 90000, 'function:_checkConcurrentMod:ordinary:unbound:bytecode': 90000, 'wide': 90000} |
| B17:pc5:wide:method | {'executions': 90000, 'function:hasNext:ordinary:unbound:bytecode': 90000, 'wide': 90000} |
| B2:pc107:vm:method | {'executions': 1, 'function:runIteration:ordinary:unbound:bytecode': 1, 'vm-direct-or-generic': 1} |
| B3:pc32:wide:plain | {'executions': 1, 'function:calculateCapacity:ordinary:unbound:bytecode': 1, 'numeric': 1} |
| B3:pc46:wide:method | {'executions': 1, 'function:_computeThreshold:ordinary:unbound:bytecode': 1, 'wide': 1} |
| B5:pc107:vm:method | {'executions': 1, 'function:hasNext:ordinary:unbound:bytecode': 1, 'vm-direct-or-generic': 1} |
| B5:pc113:vm:method | {'executions': 1, 'function:next:ordinary:unbound:bytecode': 1, 'vm-direct-or-generic': 1} |
| B5:pc12:wide:method | {'executions': 89999, 'function:put:ordinary:unbound:bytecode': 89999, 'wide': 89999} |
| B5:pc22:vm:method | {'executions': 1, 'function:put:ordinary:unbound:bytecode': 1, 'vm-direct-or-generic': 1} |
| B5:pc33:wide:method | {'executions': 449991, 'function:get:ordinary:unbound:bytecode': 449991, 'wide': 449991} |
| B5:pc51:wide:method | {'executions': 1, 'function:entrySet:ordinary:unbound:bytecode': 1, 'wide': 1} |
| B5:pc53:wide:method | {'executions': 1, 'function:iterator:ordinary:unbound:bytecode': 1, 'wide': 1} |
| B5:pc59:wide:method | {'executions': 90000, 'function:hasNext:ordinary:unbound:bytecode': 90000, 'wide': 90000} |
| B5:pc64:vm:method | {'executions': 9, 'function:get:ordinary:unbound:bytecode': 9, 'vm-direct-or-generic': 9} |
| B5:pc64:wide:method | {'executions': 89999, 'function:next:ordinary:unbound:bytecode': 89999, 'wide': 89999} |
| B6:pc14:wide:method | {'executions': 90000, 'function:_findKeyEntry:ordinary:unbound:bytecode': 90000, 'wide': 90000} |
| B6:pc2:wide:plain | {'executions': 90000, 'function:computeHashCode:ordinary:unbound:bytecode': 90000, 'numeric': 90000} |
| B6:pc36:wide:method | {'executions': 90000, 'function:_createHashedEntry:ordinary:unbound:bytecode': 90000, 'wide': 90000} |
| B6:pc52:wide:method | {'executions': 17, 'function:_rehash:ordinary:unbound:bytecode': 17, 'wide': 17} |
| B7:pc13:wide:plain | {'executions': 450000, 'function:equals:ordinary:unbound:bytecode': 450000, 'numeric': 450000} |
| B9:pc19:wide:plain | {'executions': 17, 'function:calculateCapacity:ordinary:unbound:bytecode': 17, 'numeric': 17} |
| B9:pc79:wide:method | {'executions': 17, 'function:_computeThreshold:ordinary:unbound:bytecode': 17, 'wide': 17} |
### cdjs

Totals: {"reads": 10595510, "hits": 8650996, "misses": 1944514, "method_site_reads": 2035849, "method_site_hits": 924713, "method_site_misses": 1111136}.

| Site | Executions / hot / miss / method | Priced excess instructions | Fallback counts @ R/NG instructions | Reasons; guards | Ordered / unordered / shared-name / receivers; depth; cache events |
| --- | --- | ---: | --- | --- | --- |
| B11:anonymous(other)/ops10/pc1/localeCompare | 596719/0/596719/596719 | 57543168 | primitive-prototype:596719@216.5/120.0i | {'eligibility-no-hot-probe': 596719}; {} | 0/0/0/0; depth={}; events={'update': 1} |
| B10:anonymous(key,value)/ops99/pc6/compareTo | 541391/501737/39654/541391 | 39003663 | general-prototype-chain:2@437.5/69.1i; hot-PrototypeSlot:501737@117.6/69.0i; prototype-own-miss:39652@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 39652, 'receiver-proof-absent': 1}; {'holder-not-cached': 1, 'hot-holder-differs': 36, 'hot-holder-matches': 39617, 'receiver-proof-invalid': 39653, 'valid-holder-already-cached': 39652} | 2/2/2/22653; depth={'1': 541391}; events={'update_from_prototype': 2} |
| B19:compare(a,b)/ops9/pc0/compareTo | 319297/288967/30330/319297 | 25223863 | general-prototype-chain:2@437.5/69.1i; hot-PrototypeSlot:288967@117.6/69.0i; prototype-own-miss:30328@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 30328, 'receiver-proof-absent': 1}; {'holder-not-cached': 1, 'hot-holder-differs': 36, 'hot-holder-matches': 30293, 'receiver-proof-invalid': 30329, 'valid-holder-already-cached': 30328} | 2/2/2/13329; depth={'1': 319297}; events={'update_from_prototype': 2} |
| B9:root(key,value)/ops194/pc1/_treeInsert | 69518/10658/58860/69518 | 22204753 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:10658@117.6/69.0i; prototype-own-miss:58859@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 58858, 'receiver-proof-absent': 1}; {'hot-holder-matches': 58859, 'receiver-proof-invalid': 58859, 'valid-holder-already-cached': 58859} | 1/1/1/6049; depth={'1': 69518}; events={'update_from_prototype': 1} |
| B11:anonymous(other)/ops10/pc2/_value | 596719/596712/7/0 | 15584714 | hot-SharedSlot:596712@90.0/63.9i; own-lookup:2@unpriced; probe-own-OwnSlot:4@246.0/64.0i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 6}; {} | 1/1/1/1000; depth={'0': 596719}; events={'update': 2} |
| B11:anonymous(other)/ops10/pc0/_value | 596719/596713/6/0 | 15584558 | hot-SharedSlot:596713@90.0/63.9i; own-lookup:2@unpriced; probe-own-OwnSlot:3@246.0/64.0i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 5}; {} | 1/1/1/1000; depth={'0': 596719}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc2/MAX_X | 116776/0/116776/0 | 14156738 | own-lookup:1@unpriced; probe-own-Exact:116775@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 116775}; {} | 1/1/1/1; depth={'0': 116776}; events={'update': 1} |
| B25:isInVoxel(voxel)/ops310/pc8/MIN_X | 116776/0/116776/0 | 14156738 | own-lookup:1@unpriced; probe-own-Exact:116775@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 116775}; {} | 1/1/1/1; depth={'0': 116776}; events={'update': 1} |
| B10:anonymous(key,value)/ops99/pc7/key | 541391/541388/3/0 | 14139120 | hot-SharedSlot:541388@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/33176; depth={'0': 541391}; events={'update': 2} |
| B14:root(callback)/ops40/pc21/successor | 26842/0/26842/26842 | 10593349 | general-prototype-chain:1@455.5/60.9i; prototype-own-miss:26841@455.5/60.9i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 26840, 'receiver-proof-absent': 1}; {'hot-holder-matches': 26841, 'receiver-proof-invalid': 26841, 'valid-holder-already-cached': 26841} | 1/1/1/9842; depth={'1': 26842}; events={'update_from_prototype': 1} |
| B9:root(key,value)/ops194/pc6/isNewEntry | 69518/0/69518/0 | 10545524 | own-lookup:2@unpriced; probe-own-LiteralShape:69516@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 69517}; {} | 2/2/2/69518; depth={'0': 69518}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc14/MAX_Y | 85738/0/85738/0 | 10393973 | own-lookup:1@unpriced; probe-own-Exact:85737@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 85737}; {} | 1/1/1/1; depth={'0': 85738}; events={'update': 1} |
| B10:anonymous(key,value)/ops99/pc52/compareTo | 34140/6832/27308/34140 | 10393612 | general-prototype-chain:2@437.5/69.1i; hot-PrototypeSlot:6832@117.6/69.0i; prototype-own-miss:27306@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 27306, 'receiver-proof-absent': 1}; {'holder-not-cached': 1, 'hot-holder-differs': 36, 'hot-holder-matches': 27271, 'receiver-proof-invalid': 27307, 'valid-holder-already-cached': 27306} | 2/2/2/10323; depth={'1': 34140}; events={'update_from_prototype': 2} |
| B10:anonymous(key,value)/ops99/pc19/right | 385682/385679/3/0 | 10072558 | hot-SharedSlot:385679@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/29964; depth={'0': 385682}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc20/MIN_Y | 73746/0/73746/0 | 8940172 | own-lookup:1@unpriced; probe-own-Exact:73745@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 73745}; {} | 1/1/1/1; depth={'0': 73746}; events={'update': 1} |
| B25:isInVoxel(voxel)/ops310/pc36/PROXIMITY_RADIUS | 73660/0/73660/0 | 8929747 | own-lookup:1@unpriced; probe-own-Exact:73659@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 73659}; {} | 1/1/1/1; depth={'0': 73660}; events={'update': 1} |
| B18:anonymous(key)/ops49/pc6/key | 319297/319294/3/0 | 8338819 | hot-SharedSlot:319294@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/25744; depth={'0': 319297}; events={'update': 2} |
| B29:anonymous(other)/ops23/pc2/x | 298109/298097/12/0 | 7786320 | hot-SharedSlot:298097@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:9@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 11}; {} | 1/1/1/10747; depth={'0': 298109}; events={'update': 2} |
| B29:anonymous(other)/ops23/pc1/x | 298109/298102/7/0 | 7785844 | hot-SharedSlot:298102@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:4@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 6}; {} | 1/1/1/24658; depth={'0': 298109}; events={'update': 2} |
| B29:anonymous(other)/ops23/pc12/y | 286600/286588/12/0 | 7485746 | hot-SharedSlot:286588@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:9@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 11}; {} | 1/1/1/10747; depth={'0': 286600}; events={'update': 2} |
| B29:anonymous(other)/ops23/pc11/y | 286600/286593/7/0 | 7485270 | hot-SharedSlot:286593@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:4@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 6}; {} | 1/1/1/24655; depth={'0': 286600}; events={'update': 2} |
| B8:vm(frame)/ops222/pc33/put | 18000/0/18000/18000 | 6631983 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:17999@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 17999}; {'hot-holder-matches': 17999, 'receiver-proof-invalid': 17999, 'valid-holder-already-cached': 17999} | 1/1/1/18; depth={'1': 18000}; events={'update_from_prototype': 1} |
| B8:vm(frame)/ops222/pc24/put | 18000/0/18000/18000 | 6631983 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:17999@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 17999}; {'hot-holder-matches': 17999, 'receiver-proof-invalid': 17999, 'valid-holder-already-cached': 17999} | 1/1/1/1; depth={'1': 18000}; events={'update_from_prototype': 1} |
| B9:root(key,value)/ops194/pc14/newNode | 40189/0/40189/0 | 6096489 | own-lookup:1@unpriced; probe-own-LiteralShape:40188@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 40188}; {} | 1/1/1/40189; depth={'0': 40189}; events={'update': 1} |
| B18:anonymous(key)/ops49/pc23/right | 227506/227503/3/0 | 5941566 | hot-SharedSlot:227503@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/16738; depth={'0': 227506}; events={'update': 2} |
| B24:recurse(nextVoxel)/ops103/pc26/minus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc32/plus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc38/minus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc81/plus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc77/plus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc44/plus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc72/minus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc68/plus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc63/plus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc59/minus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc54/minus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B24:recurse(nextVoxel)/ops103/pc50/minus | 12347/0/12347/12347 | 4549172 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:12346@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 12345, 'receiver-proof-absent': 1}; {'hot-holder-matches': 12346, 'receiver-proof-invalid': 12346, 'valid-holder-already-cached': 12346} | 1/1/1/12347; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B9:anonymous(key,value)/ops194/pc10/oldValue | 29329/0/29329/0 | 4449035 | own-lookup:1@unpriced; probe-own-LiteralShape:29328@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 29328}; {} | 1/1/1/29329; depth={'0': 29329}; events={'update': 1} |
| B10:anonymous(key,value)/ops99/pc13/left | 126380/126377/3/0 | 3300516 | hot-SharedSlot:126377@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/6875; depth={'0': 126380}; events={'update': 2} |
| B24:recurse(nextVoxel)/ops103/pc12/put | 24676/18663/6013/24676 | 3122807 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:18663@117.6/69.0i; prototype-own-miss:6012@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 6011, 'receiver-proof-absent': 1}; {'hot-holder-matches': 6012, 'receiver-proof-invalid': 6012, 'valid-holder-already-cached': 6012} | 1/1/1/6012; depth={'1': 24676}; events={'update_from_prototype': 1} |
| B25:isInVoxel(voxel)/ops310/pc0/x | 116776/116773/3/0 | 3049694 | hot-SharedSlot:116773@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/116776; depth={'0': 116776}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc6/x | 116776/116773/3/0 | 3049694 | hot-SharedSlot:116773@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/116776; depth={'0': 116776}; events={'update': 2} |
| B8:vm(frame)/ops222/pc25/callsign | 18000/0/18000/0 | 2730283 | own-lookup:2@unpriced; probe-own-LiteralShape:17998@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 17999}; {} | 1/1/2/18000; depth={'0': 18000}; events={'update': 2} |
| B8:vm(frame)/ops222/pc26/position | 18000/0/18000/0 | 2730283 | own-lookup:2@unpriced; probe-own-LiteralShape:17998@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 17999}; {} | 1/1/2/18000; depth={'0': 18000}; events={'update': 2} |
| B8:vm(frame)/ops222/pc29/position | 18000/0/18000/0 | 2730283 | own-lookup:2@unpriced; probe-own-LiteralShape:17998@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 17999}; {} | 1/1/2/18000; depth={'0': 18000}; events={'update': 2} |
| B8:vm(frame)/ops222/pc34/callsign | 18000/0/18000/0 | 2730283 | own-lookup:2@unpriced; probe-own-LiteralShape:17998@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 17999}; {} | 1/1/2/18000; depth={'0': 18000}; events={'update': 2} |
| B8:vm(frame)/ops222/pc53/callsign | 18000/0/18000/0 | 2730283 | own-lookup:2@unpriced; probe-own-LiteralShape:17998@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 17999}; {} | 1/1/2/18000; depth={'0': 18000}; events={'update': 2} |
| B34:anonymous()/ops9/pc1/dot | 7010/0/7010/7010 | 2582789 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:7009@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 7008, 'receiver-proof-absent': 1}; {'hot-holder-matches': 7009, 'receiver-proof-invalid': 7009, 'valid-holder-already-cached': 7009} | 1/1/1/7010; depth={'1': 7010}; events={'update_from_prototype': 1} |
| B32:anonymous()/ops11/pc1/minus | 7010/0/7010/7010 | 2582789 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:7009@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 7008, 'receiver-proof-absent': 1}; {'hot-holder-matches': 7009, 'receiver-proof-invalid': 7009, 'valid-holder-already-cached': 7009} | 1/1/1/6010; depth={'1': 7010}; events={'update_from_prototype': 1} |
| B9:root(key,value)/ops194/pc17/_root | 89290/89286/4/0 | 2331953 | hot-SharedSlot:89286@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/6049; depth={'0': 89290}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc12/y | 85738/85735/3/0 | 2239092 | hot-SharedSlot:85735@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/85738; depth={'0': 85738}; events={'update': 2} |
| B8:vm(frame)/ops222/pc22/_state | 18000/0/18000/0 | 2182035 | own-lookup:1@unpriced; probe-own-Exact:17999@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 17999}; {} | 1/1/1/1; depth={'0': 18000}; events={'update': 1} |
| B9:root(key,value)/ops194/pc22/color | 81998/81995/3/0 | 2141416 | hot-SharedSlot:81995@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 2}; {} | 1/1/1/31536; depth={'0': 81998}; events={'update': 2} |
| B9:root(key,value)/ops194/pc21/parent | 81998/81995/3/0 | 2141416 | hot-SharedSlot:81995@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/34140; depth={'0': 81998}; events={'update': 2} |
| B27:anonymous(other)/ops14/pc1/x | 74082/74078/4/0 | 1934774 | hot-SharedSlot:74078@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/37041; depth={'0': 74082}; events={'update': 2} |
| B27:anonymous(other)/ops14/pc4/y | 74082/74078/4/0 | 1934774 | hot-SharedSlot:74078@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/37041; depth={'0': 74082}; events={'update': 2} |
| B28:anonymous(other)/ops14/pc2/x | 74082/74078/4/0 | 1934774 | hot-SharedSlot:74078@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/2; depth={'0': 74082}; events={'update': 2} |
| B28:anonymous(other)/ops14/pc5/y | 74082/74078/4/0 | 1934774 | hot-SharedSlot:74078@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/2; depth={'0': 74082}; events={'update': 2} |
| B27:anonymous(other)/ops14/pc2/x | 74082/74078/4/0 | 1934774 | hot-SharedSlot:74078@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/2; depth={'0': 74082}; events={'update': 2} |
| B27:anonymous(other)/ops14/pc5/y | 74082/74078/4/0 | 1934774 | hot-SharedSlot:74078@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/2; depth={'0': 74082}; events={'update': 2} |
| B28:anonymous(other)/ops14/pc1/x | 74082/74079/3/0 | 1934679 | hot-SharedSlot:74079@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/37041; depth={'0': 74082}; events={'update': 2} |
| B28:anonymous(other)/ops14/pc4/y | 74082/74079/3/0 | 1934679 | hot-SharedSlot:74079@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/37041; depth={'0': 74082}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc18/y | 73746/73743/3/0 | 1925904 | hot-SharedSlot:73743@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/73746; depth={'0': 73746}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc29/posOne | 73660/73649/11/0 | 1924419 | hot-SharedSlot:73649@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:8@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 10}; {} | 1/1/1/6012; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc32/posTwo | 73660/73649/11/0 | 1924419 | hot-SharedSlot:73649@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:8@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 10}; {} | 1/1/1/6012; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc40/x | 73660/73649/11/0 | 1924419 | hot-SharedSlot:73649@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:8@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 10}; {} | 1/1/1/5678; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc41/x | 73660/73649/11/0 | 1924419 | hot-SharedSlot:73649@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:8@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 10}; {} | 1/1/1/6012; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc42/x | 73660/73649/11/0 | 1924419 | hot-SharedSlot:73649@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:8@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 10}; {} | 1/1/1/5678; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc45/y | 73660/73649/11/0 | 1924419 | hot-SharedSlot:73649@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:8@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 10}; {} | 1/1/1/5678; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc46/y | 73660/73649/11/0 | 1924419 | hot-SharedSlot:73649@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:8@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 10}; {} | 1/1/1/6012; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc47/y | 73660/73649/11/0 | 1924419 | hot-SharedSlot:73649@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:8@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 10}; {} | 1/1/1/5678; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc39/x | 73660/73657/3/0 | 1923658 | hot-SharedSlot:73657@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/73660; depth={'0': 73660}; events={'update': 2} |
| B25:isInVoxel(voxel)/ops310/pc44/y | 73660/73657/3/0 | 1923658 | hot-SharedSlot:73657@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/73660; depth={'0': 73660}; events={'update': 2} |
| B18:anonymous(key)/ops49/pc19/left | 70286/70283/3/0 | 1835541 | hot-SharedSlot:70283@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/10813; depth={'0': 70286}; events={'update': 2} |
| B10:anonymous(key,value)/ops99/pc1/_root | 69518/69515/3/0 | 1815483 | hot-SharedSlot:69515@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/6049; depth={'0': 69518}; events={'update': 2} |
| B9:root(key,value)/ops194/pc146/_leftRotate | 16937/14015/2922/16937 | 1757972 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:14015@117.6/69.0i; prototype-own-miss:2921@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 2920, 'receiver-proof-absent': 1}; {'hot-holder-matches': 2921, 'receiver-proof-invalid': 2921, 'valid-holder-already-cached': 2921} | 1/1/1/1036; depth={'1': 16937}; events={'update_from_prototype': 1} |
| B17:anonymous(key)/ops20/pc1/_findNode | 30347/30310/37/30347 | 1487241 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:30310@117.6/69.0i; prototype-own-miss:36@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 35, 'receiver-proof-absent': 1}; {'hot-holder-matches': 36, 'receiver-proof-invalid': 36, 'valid-holder-already-cached': 36} | 1/1/1/36; depth={'1': 30347}; events={'update_from_prototype': 1} |
| B7:anonymous(time)/ops83/pc4/_aircraft | 9018/0/9018/0 | 1413255 | own-lookup:1@unpriced; probe-own-OwnSlot:9017@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 9017}; {} | 1/1/1/1; depth={'0': 9018}; events={'update': 1} |
| B7:anonymous(time)/ops83/pc28/_aircraft | 9000/0/9000/0 | 1410434 | own-lookup:1@unpriced; probe-own-OwnSlot:8999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 8999}; {} | 1/1/1/1; depth={'0': 9000}; events={'update': 1} |
| B7:anonymous(time)/ops83/pc8/_aircraft | 9000/0/9000/0 | 1410434 | own-lookup:1@unpriced; probe-own-OwnSlot:8999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 8999}; {} | 1/1/1/1; depth={'0': 9000}; events={'update': 1} |
| B31:root(other)/ops267/pc3/delta | 3505/0/3505/3505 | 1291394 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3504@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3503, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3504, 'receiver-proof-invalid': 3504, 'valid-holder-already-cached': 3504} | 1/1/1/3005; depth={'1': 3505}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc6/delta | 3505/0/3505/3505 | 1291394 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3504@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3503, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3504, 'receiver-proof-invalid': 3504, 'valid-holder-already-cached': 3504} | 1/1/1/3005; depth={'1': 3505}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc13/minus | 3505/0/3505/3505 | 1291394 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3504@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3503, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3504, 'receiver-proof-invalid': 3504, 'valid-holder-already-cached': 3504} | 1/1/1/3505; depth={'1': 3505}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc17/squaredMagnitude | 3505/0/3505/3505 | 1291394 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3504@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3503, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3504, 'receiver-proof-invalid': 3504, 'valid-holder-already-cached': 3504} | 1/1/1/3505; depth={'1': 3505}; events={'update_from_prototype': 1} |
| B8:vm(frame)/ops222/pc162/findIntersection | 3505/0/3505/3505 | 1291394 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:3504@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 3504}; {'hot-holder-matches': 3504, 'receiver-proof-invalid': 3504, 'valid-holder-already-cached': 3504} | 1/1/1/3005; depth={'1': 3505}; events={'update_from_prototype': 1} |
| B9:root(key,value)/ops194/pc29/parent | 49105/49102/3/0 | 1282369 | hot-SharedSlot:49102@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/27708; depth={'0': 49105}; events={'update': 2} |
| B9:root(key,value)/ops194/pc30/left | 49105/49102/3/0 | 1282369 | hot-SharedSlot:49102@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/22717; depth={'0': 49105}; events={'update': 2} |
| B9:root(key,value)/ops194/pc27/parent | 49105/49102/3/0 | 1282369 | hot-SharedSlot:49102@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/28047; depth={'0': 49105}; events={'update': 2} |
| B9:root(key,value)/ops194/pc28/parent | 49105/49102/3/0 | 1282369 | hot-SharedSlot:49102@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/28047; depth={'0': 49105}; events={'update': 2} |
| B31:root(other)/ops267/pc23/minus | 3338/0/3338/3338 | 1229864 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3337@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3336, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3337, 'receiver-proof-invalid': 3337, 'valid-holder-already-cached': 3337} | 1/1/1/2838; depth={'1': 3338}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc27/dot | 3338/0/3338/3338 | 1229864 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3337@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3336, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3337, 'receiver-proof-invalid': 3337, 'valid-holder-already-cached': 3337} | 1/1/1/3338; depth={'1': 3338}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc28/minus | 3338/0/3338/3338 | 1229864 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3337@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3336, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3337, 'receiver-proof-invalid': 3337, 'valid-holder-already-cached': 3337} | 1/1/1/3338; depth={'1': 3338}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc37/minus | 3338/0/3338/3338 | 1229864 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3337@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3336, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3337, 'receiver-proof-invalid': 3337, 'valid-holder-already-cached': 3337} | 1/1/1/2838; depth={'1': 3338}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc41/squaredMagnitude | 3338/0/3338/3338 | 1229864 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:3337@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 3336, 'receiver-proof-absent': 1}; {'hot-holder-matches': 3337, 'receiver-proof-invalid': 3337, 'valid-holder-already-cached': 3337} | 1/1/1/3338; depth={'1': 3338}; events={'update_from_prototype': 1} |
| B26:putIntoMap(voxel)/ops29/pc18/push | 12347/0/12347/12347 | 1190653 | primitive-prototype:12347@216.5/120.0i | {'eligibility-no-hot-probe': 12347}; {} | 0/0/0/0; depth={}; events={'update': 1} |
| B7:anonymous(time)/ops83/pc36/sin | 9000/0/9000/9000 | 1090957 | own-lookup:1@unpriced; probe-own-Exact:8999@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 8999}; {} | 1/1/1/1; depth={'0': 9000}; events={'update': 1} |
| B7:anonymous(time)/ops83/pc14/cos | 9000/0/9000/9000 | 1090957 | own-lookup:1@unpriced; probe-own-Exact:8999@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 8999}; {} | 1/1/1/1; depth={'0': 9000}; events={'update': 1} |
| B9:root(key,value)/ops194/pc157/_root | 40189/40185/4/0 | 1049610 | hot-SharedSlot:40185@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/6049; depth={'0': 40189}; events={'update': 2} |
| B9:root(key,value)/ops194/pc94/parent | 39253/39250/3/0 | 1025070 | hot-SharedSlot:39250@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/20545; depth={'0': 39253}; events={'update': 2} |
| B9:root(key,value)/ops194/pc95/left | 39253/39250/3/0 | 1025070 | hot-SharedSlot:39250@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/21707; depth={'0': 39253}; events={'update': 2} |
| B9:root(key,value)/ops194/pc93/parent | 39253/39250/3/0 | 1025070 | hot-SharedSlot:39250@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/25186; depth={'0': 39253}; events={'update': 2} |
| B10:anonymous(key,value)/ops99/pc53/key | 34140/34137/3/0 | 891536 | hot-SharedSlot:34137@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/25899; depth={'0': 34140}; events={'update': 2} |
| B18:anonymous(key)/ops49/pc0/_root | 30347/29345/1002/0 | 887496 | hot-SharedSlot:29345@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:999@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 1001}; {} | 1/1/1/36; depth={'0': 30347}; events={'update': 2} |
| B16:anonymous(callsign,position)/ops19/pc1/get | 18000/17981/19/18000 | 881199 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:17981@117.6/69.0i; prototype-own-miss:18@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 17, 'receiver-proof-absent': 1}; {'hot-holder-matches': 18, 'receiver-proof-invalid': 18, 'valid-holder-already-cached': 18} | 1/1/1/18; depth={'1': 18000}; events={'update_from_prototype': 1} |
| B7:anonymous(time)/ops83/pc27/push | 9000/0/9000/9000 | 867893 | primitive-prototype:9000@216.5/120.0i | {'eligibility-no-hot-probe': 9000}; {} | 0/0/0/0; depth={}; events={'update': 1} |
| B7:anonymous(time)/ops83/pc7/push | 9000/0/9000/9000 | 867893 | primitive-prototype:9000@216.5/120.0i | {'eligibility-no-hot-probe': 9000}; {} | 0/0/0/0; depth={}; events={'update': 1} |
| B9:root(key,value)/ops194/pc99/color | 29717/29714/3/0 | 776024 | hot-SharedSlot:29714@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 2}; {} | 1/1/1/21732; depth={'0': 29717}; events={'update': 2} |
| B10:anonymous(key,value)/ops99/pc23/value | 29329/29326/3/0 | 765890 | hot-SharedSlot:29326@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/8334; depth={'0': 29329}; events={'update': 2} |
| B20:anonymous()/ops38/pc1/right | 26842/26839/3/0 | 700939 | hot-SharedSlot:26839@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/9842; depth={'0': 26842}; events={'update': 2} |
| B14:root(callback)/ops40/pc16/key | 26842/26839/3/0 | 700939 | hot-SharedSlot:26839@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/9842; depth={'0': 26842}; events={'update': 2} |
| B14:root(callback)/ops40/pc17/value | 26842/26839/3/0 | 700939 | hot-SharedSlot:26839@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/9842; depth={'0': 26842}; events={'update': 2} |
| B15:treeMinimum(x)/ops15/pc2/left | 26711/26708/3/0 | 697518 | hot-SharedSlot:26708@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/9842; depth={'0': 26711}; events={'update': 2} |
| B20:anonymous()/ops38/pc16/right | 26684/26680/4/0 | 696908 | hot-SharedSlot:26680@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/5249; depth={'0': 26684}; events={'update': 2} |
| B9:root(key,value)/ops194/pc85/_rightRotate | 7849/7004/845/7849 | 651855 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:7004@117.6/69.0i; prototype-own-miss:844@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 843, 'receiver-proof-absent': 1}; {'hot-holder-matches': 844, 'receiver-proof-invalid': 844, 'valid-holder-already-cached': 844} | 1/1/1/21; depth={'1': 7849}; events={'update_from_prototype': 1} |
| B12:anonymous(x)/ops53/pc0/right | 24689/24686/3/0 | 644710 | hot-SharedSlot:24686@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/23318; depth={'0': 24689}; events={'update': 2} |
| B12:anonymous(x)/ops53/pc1/left | 24689/24686/3/0 | 644710 | hot-SharedSlot:24686@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/14308; depth={'0': 24689}; events={'update': 2} |
| B12:anonymous(x)/ops53/pc4/left | 24689/24686/3/0 | 644710 | hot-SharedSlot:24686@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/14308; depth={'0': 24689}; events={'update': 2} |
| B12:anonymous(x)/ops53/pc13/parent | 24689/24686/3/0 | 644710 | hot-SharedSlot:24686@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/23318; depth={'0': 24689}; events={'update': 2} |
| B12:anonymous(x)/ops53/pc16/parent | 24689/24686/3/0 | 644710 | hot-SharedSlot:24686@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/23318; depth={'0': 24689}; events={'update': 2} |
| B9:root(key,value)/ops194/pc67/_leftRotate | 7752/6919/833/7752 | 643301 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:6919@117.6/69.0i; prototype-own-miss:832@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 831, 'receiver-proof-absent': 1}; {'hot-holder-matches': 832, 'receiver-proof-invalid': 832, 'valid-holder-already-cached': 832} | 1/1/1/19; depth={'1': 7752}; events={'update_from_prototype': 1} |
| B12:anonymous(x)/ops53/pc25/left | 23470/23467/3/0 | 612874 | hot-SharedSlot:23467@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/10224; depth={'0': 23470}; events={'update': 2} |
| B12:anonymous(x)/ops53/pc24/parent | 23470/23467/3/0 | 612874 | hot-SharedSlot:23467@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/22159; depth={'0': 23470}; events={'update': 2} |
| B26:putIntoMap(voxel)/ops29/pc1/get | 12347/12328/19/12347 | 606362 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:12328@117.6/69.0i; prototype-own-miss:18@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 17, 'receiver-proof-absent': 1}; {'hot-holder-matches': 18, 'receiver-proof-invalid': 18, 'valid-holder-already-cached': 18} | 1/1/1/18; depth={'1': 12347}; events={'update_from_prototype': 1} |
| B9:root(key,value)/ops194/pc113/parent | 22316/22313/3/0 | 582736 | hot-SharedSlot:22313@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/14762; depth={'0': 22316}; events={'update': 2} |
| B9:root(key,value)/ops194/pc118/parent | 22316/22313/3/0 | 582736 | hot-SharedSlot:22313@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/14762; depth={'0': 22316}; events={'update': 2} |
| B9:root(key,value)/ops194/pc104/parent | 22316/22313/3/0 | 582736 | hot-SharedSlot:22313@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/15015; depth={'0': 22316}; events={'update': 2} |
| B9:root(key,value)/ops194/pc112/parent | 22316/22313/3/0 | 582736 | hot-SharedSlot:22313@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/15015; depth={'0': 22316}; events={'update': 2} |
| B9:root(key,value)/ops194/pc117/parent | 22316/22313/3/0 | 582736 | hot-SharedSlot:22313@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/15015; depth={'0': 22316}; events={'update': 2} |
| B17:anonymous(key)/ops20/pc12/value | 21505/21502/3/0 | 561555 | hot-SharedSlot:21502@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/21505; depth={'0': 21505}; events={'update': 2} |
| B33:anonymous(other)/ops18/pc1/x | 20696/20693/3/0 | 540427 | hot-SharedSlot:20693@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/12853; depth={'0': 20696}; events={'update': 2} |
| B33:anonymous(other)/ops18/pc4/y | 20696/20693/3/0 | 540427 | hot-SharedSlot:20693@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/12853; depth={'0': 20696}; events={'update': 2} |
| B33:anonymous(other)/ops18/pc7/z | 20696/20693/3/0 | 540427 | hot-SharedSlot:20693@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/12853; depth={'0': 20696}; events={'update': 2} |
| B33:anonymous(other)/ops18/pc2/x | 20696/20693/3/0 | 540427 | hot-SharedSlot:20693@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/12519; depth={'0': 20696}; events={'update': 2} |
| B33:anonymous(other)/ops18/pc5/y | 20696/20693/3/0 | 540427 | hot-SharedSlot:20693@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/12519; depth={'0': 20696}; events={'update': 2} |
| B33:anonymous(other)/ops18/pc8/z | 20696/20693/3/0 | 540427 | hot-SharedSlot:20693@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/12519; depth={'0': 20696}; events={'update': 2} |
| B31:root(other)/ops267/pc112/plus | 1336/0/1336/1336 | 492240 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:1335@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 1334, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1335, 'receiver-proof-invalid': 1335, 'valid-holder-already-cached': 1335} | 1/1/1/1336; depth={'1': 1336}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc113/times | 1336/0/1336/1336 | 492240 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:1335@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 1334, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1335, 'receiver-proof-invalid': 1335, 'valid-holder-already-cached': 1335} | 1/1/1/1336; depth={'1': 1336}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc120/plus | 1336/0/1336/1336 | 492240 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:1335@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 1334, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1335, 'receiver-proof-invalid': 1335, 'valid-holder-already-cached': 1335} | 1/1/1/1336; depth={'1': 1336}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc121/times | 1336/0/1336/1336 | 492240 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:1335@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 1334, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1335, 'receiver-proof-invalid': 1335, 'valid-holder-already-cached': 1335} | 1/1/1/1336; depth={'1': 1336}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc128/plus | 1336/0/1336/1336 | 492240 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:1335@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 1334, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1335, 'receiver-proof-invalid': 1335, 'valid-holder-already-cached': 1335} | 1/1/1/1336; depth={'1': 1336}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc132/times | 1336/0/1336/1336 | 492240 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:1335@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 1334, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1335, 'receiver-proof-invalid': 1335, 'valid-holder-already-cached': 1335} | 1/1/1/1336; depth={'1': 1336}; events={'update_from_prototype': 1} |
| B23:voxelHash(position)/ops64/pc33/y | 18000/17997/3/0 | 470017 | hot-SharedSlot:17997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/17000; depth={'0': 18000}; events={'update': 2} |
| B23:voxelHash(position)/ops64/pc20/x | 18000/17997/3/0 | 470017 | hot-SharedSlot:17997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/17000; depth={'0': 18000}; events={'update': 2} |
| B23:voxelHash(position)/ops64/pc5/y | 18000/17997/3/0 | 470017 | hot-SharedSlot:17997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/17000; depth={'0': 18000}; events={'update': 2} |
| B23:voxelHash(position)/ops64/pc0/x | 18000/17997/3/0 | 470017 | hot-SharedSlot:17997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/17000; depth={'0': 18000}; events={'update': 2} |
| B22:vm(voxelMap,motion)/ops26/pc21/posOne | 18000/17997/3/0 | 470017 | hot-SharedSlot:17997@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/18000; depth={'0': 18000}; events={'update': 2} |
| B9:root(key,value)/ops194/pc124/left | 16937/16934/3/0 | 442256 | hot-SharedSlot:16934@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/11542; depth={'0': 16937}; events={'update': 2} |
| B9:root(key,value)/ops194/pc141/parent | 16937/16934/3/0 | 442256 | hot-SharedSlot:16934@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/11173; depth={'0': 16937}; events={'update': 2} |
| B9:root(key,value)/ops194/pc148/parent | 16937/16934/3/0 | 442256 | hot-SharedSlot:16934@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/11173; depth={'0': 16937}; events={'update': 2} |
| B9:root(key,value)/ops194/pc123/parent | 16937/16934/3/0 | 442256 | hot-SharedSlot:16934@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/15666; depth={'0': 16937}; events={'update': 2} |
| B9:root(key,value)/ops194/pc136/parent | 16937/16934/3/0 | 442256 | hot-SharedSlot:16934@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/16394; depth={'0': 16937}; events={'update': 2} |
| B9:root(key,value)/ops194/pc140/parent | 16937/16934/3/0 | 442256 | hot-SharedSlot:16934@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/16394; depth={'0': 16937}; events={'update': 2} |
| B9:root(key,value)/ops194/pc147/parent | 16937/16934/3/0 | 442256 | hot-SharedSlot:16934@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/16394; depth={'0': 16937}; events={'update': 2} |
| B26:putIntoMap(voxel)/ops29/pc9/put | 8842/8823/19/8842 | 435956 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:8823@117.6/69.0i; prototype-own-miss:18@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 17, 'receiver-proof-absent': 1}; {'hot-holder-matches': 18, 'receiver-proof-invalid': 18, 'valid-holder-already-cached': 18} | 1/1/1/18; depth={'1': 8842}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc11/PROXIMITY_RADIUS | 3505/0/3505/0 | 424793 | own-lookup:1@unpriced; probe-own-Exact:3504@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3504}; {} | 1/1/1/1; depth={'0': 3505}; events={'update': 1} |
| B31:root(other)/ops267/pc59/sqrt | 3338/0/3338/3338 | 404548 | own-lookup:1@unpriced; probe-own-Exact:3337@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3337}; {} | 1/1/1/1; depth={'0': 3338}; events={'update': 1} |
| B31:root(other)/ops267/pc69/sqrt | 3338/0/3338/3338 | 404548 | own-lookup:1@unpriced; probe-own-Exact:3337@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3337}; {} | 1/1/1/1; depth={'0': 3338}; events={'update': 1} |
| B12:anonymous(x)/ops53/pc32/parent | 13778/13775/3/0 | 359754 | hot-SharedSlot:13775@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/13303; depth={'0': 13778}; events={'update': 2} |
| B20:anonymous()/ops38/pc5/right | 13749/13746/3/0 | 358996 | hot-SharedSlot:13746@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/5249; depth={'0': 13749}; events={'update': 2} |
| B20:anonymous()/ops38/pc21/parent | 13691/13688/3/0 | 357482 | hot-SharedSlot:13688@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/5249; depth={'0': 13691}; events={'update': 2} |
| B20:anonymous()/ops38/pc10/parent | 13093/13090/3/0 | 341864 | hot-SharedSlot:13090@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4593; depth={'0': 13093}; events={'update': 2} |
| B15:treeMinimum(x)/ops15/pc5/left | 12990/12987/3/0 | 339174 | hot-SharedSlot:12987@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4574; depth={'0': 12990}; events={'update': 2} |
| B30:anonymous(key,value)/ops16/pc4/push | 3505/0/3505/3505 | 337996 | primitive-prototype:3505@216.5/120.0i | {'eligibility-no-hot-probe': 3505}; {} | 0/0/0/0; depth={}; events={'update': 1} |
| B7:anonymous(time)/ops83/pc5/length | 9018/0/9018/0 | 334353 | primitive-prototype:9018@96.0/58.9i | {'eligibility-no-hot-probe': 9018}; {} | 0/0/0/0; depth={}; events={} |
| B30:anonymous(key,value)/ops16/pc0/length | 8842/0/8842/0 | 327827 | primitive-prototype:8842@96.0/58.9i | {'eligibility-no-hot-probe': 8842}; {} | 0/0/0/0; depth={}; events={} |
| B12:anonymous(x)/ops53/pc7/left | 10593/10590/3/0 | 276573 | hot-SharedSlot:10590@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/5894; depth={'0': 10593}; events={'update': 2} |
| B9:root(key,value)/ops194/pc128/_rightRotate | 1134/444/690/1134 | 275812 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:444@117.6/69.0i; prototype-own-miss:689@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 688, 'receiver-proof-absent': 1}; {'hot-holder-matches': 689, 'receiver-proof-invalid': 689, 'valid-holder-already-cached': 689} | 1/1/1/356; depth={'1': 1134}; events={'update_from_prototype': 1} |
| B35:anonymous(other)/ops18/pc0/x | 10348/10345/3/0 | 270174 | hot-SharedSlot:10345@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/10348; depth={'0': 10348}; events={'update': 2} |
| B35:anonymous(other)/ops18/pc3/y | 10348/10345/3/0 | 270174 | hot-SharedSlot:10345@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/10348; depth={'0': 10348}; events={'update': 2} |
| B35:anonymous(other)/ops18/pc7/z | 10348/10345/3/0 | 270174 | hot-SharedSlot:10345@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/10348; depth={'0': 10348}; events={'update': 2} |
| B35:anonymous(other)/ops18/pc1/x | 10348/10345/3/0 | 270174 | hot-SharedSlot:10345@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/10348; depth={'0': 10348}; events={'update': 2} |
| B35:anonymous(other)/ops18/pc4/y | 10348/10345/3/0 | 270174 | hot-SharedSlot:10345@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/10348; depth={'0': 10348}; events={'update': 2} |
| B35:anonymous(other)/ops18/pc8/z | 10348/10345/3/0 | 270174 | hot-SharedSlot:10345@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/10348; depth={'0': 10348}; events={'update': 2} |
| B9:root(key,value)/ops194/pc33/parent | 9852/9849/3/0 | 257221 | hot-SharedSlot:9849@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/9700; depth={'0': 9852}; events={'update': 2} |
| B9:root(key,value)/ops194/pc34/right | 9852/9849/3/0 | 257221 | hot-SharedSlot:9849@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/3412; depth={'0': 9852}; events={'update': 2} |
| B9:root(key,value)/ops194/pc32/parent | 9852/9849/3/0 | 257221 | hot-SharedSlot:9849@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/7572; depth={'0': 9852}; events={'update': 2} |
| B12:anonymous(x)/ops53/pc27/parent | 9692/9689/3/0 | 253042 | hot-SharedSlot:9689@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/9502; depth={'0': 9692}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc0/left | 8983/8980/3/0 | 234526 | hot-SharedSlot:8980@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2770; depth={'0': 8983}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc1/right | 8983/8980/3/0 | 234526 | hot-SharedSlot:8980@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/6893; depth={'0': 8983}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc4/right | 8983/8980/3/0 | 234526 | hot-SharedSlot:8980@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/6893; depth={'0': 8983}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc13/parent | 8983/8980/3/0 | 234526 | hot-SharedSlot:8980@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2770; depth={'0': 8983}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc16/parent | 8983/8980/3/0 | 234526 | hot-SharedSlot:8980@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2770; depth={'0': 8983}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc25/left | 8945/8942/3/0 | 233533 | hot-SharedSlot:8942@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/5981; depth={'0': 8945}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc24/parent | 8945/8942/3/0 | 233533 | hot-SharedSlot:8942@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2770; depth={'0': 8945}; events={'update': 2} |
| B9:root(key,value)/ops194/pc63/right | 7849/7846/3/0 | 204909 | hot-SharedSlot:7846@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/7697; depth={'0': 7849}; events={'update': 2} |
| B9:root(key,value)/ops194/pc80/parent | 7849/7846/3/0 | 204909 | hot-SharedSlot:7846@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/5816; depth={'0': 7849}; events={'update': 2} |
| B9:root(key,value)/ops194/pc87/parent | 7849/7846/3/0 | 204909 | hot-SharedSlot:7846@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/5816; depth={'0': 7849}; events={'update': 2} |
| B9:root(key,value)/ops194/pc62/parent | 7849/7846/3/0 | 204909 | hot-SharedSlot:7846@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/5797; depth={'0': 7849}; events={'update': 2} |
| B9:root(key,value)/ops194/pc75/parent | 7849/7846/3/0 | 204909 | hot-SharedSlot:7846@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/7792; depth={'0': 7849}; events={'update': 2} |
| B9:root(key,value)/ops194/pc79/parent | 7849/7846/3/0 | 204909 | hot-SharedSlot:7846@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/7792; depth={'0': 7849}; events={'update': 2} |
| B9:root(key,value)/ops194/pc86/parent | 7849/7846/3/0 | 204909 | hot-SharedSlot:7846@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/7792; depth={'0': 7849}; events={'update': 2} |
| B9:root(key,value)/ops194/pc65/parent | 7752/7749/3/0 | 202376 | hot-SharedSlot:7749@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/5757; depth={'0': 7752}; events={'update': 2} |
| B32:anonymous()/ops11/pc0/posTwo | 7010/7007/3/0 | 182998 | hot-SharedSlot:7007@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/6010; depth={'0': 7010}; events={'update': 2} |
| B32:anonymous()/ops11/pc2/posOne | 7010/7007/3/0 | 182998 | hot-SharedSlot:7007@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/6010; depth={'0': 7010}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc32/parent | 6760/6757/3/0 | 176469 | hot-SharedSlot:6757@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2770; depth={'0': 6760}; events={'update': 2} |
| B31:root(other)/ops267/pc138/MIN_X | 1336/0/1336/0 | 161843 | own-lookup:1@unpriced; probe-own-Exact:1335@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 1335}; {} | 1/1/1/1; depth={'0': 1336}; events={'update': 1} |
| B31:root(other)/ops267/pc144/MAX_X | 1336/0/1336/0 | 161843 | own-lookup:1@unpriced; probe-own-Exact:1335@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 1335}; {} | 1/1/1/1; depth={'0': 1336}; events={'update': 1} |
| B31:root(other)/ops267/pc150/MIN_Y | 1336/0/1336/0 | 161843 | own-lookup:1@unpriced; probe-own-Exact:1335@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 1335}; {} | 1/1/1/1; depth={'0': 1336}; events={'update': 1} |
| B31:root(other)/ops267/pc156/MAX_Y | 1336/0/1336/0 | 161843 | own-lookup:1@unpriced; probe-own-Exact:1335@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 1335}; {} | 1/1/1/1; depth={'0': 1336}; events={'update': 1} |
| B31:root(other)/ops267/pc162/MIN_Z | 1336/0/1336/0 | 161843 | own-lookup:1@unpriced; probe-own-Exact:1335@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 1335}; {} | 1/1/1/1; depth={'0': 1336}; events={'update': 1} |
| B31:root(other)/ops267/pc168/MAX_Z | 1336/0/1336/0 | 161843 | own-lookup:1@unpriced; probe-own-Exact:1335@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 1335}; {} | 1/1/1/1; depth={'0': 1336}; events={'update': 1} |
| B6:Simulator(numAircraft)/ops37/pc6/_aircraft | 1000/0/1000/0 | 156576 | own-lookup:1@unpriced; probe-own-OwnSlot:999@207.7/51.0i | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 999}; {} | 1/1/1/1; depth={'0': 1000}; events={'update': 1} |
| B9:root(key,value)/ops194/pc38/color | 5290/5286/4/0 | 138233 | hot-SharedSlot:5286@90.0/63.9i; own-lookup:2@unpriced; probe-own-OwnSlot:1@246.0/64.0i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-OwnSlot': 3}; {} | 1/1/1/2706; depth={'0': 5290}; events={'update': 2} |
| B38:anonymous(other)/ops18/pc1/x | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B38:anonymous(other)/ops18/pc4/y | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B38:anonymous(other)/ops18/pc7/z | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B37:anonymous(amount)/ops18/pc1/x | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B37:anonymous(amount)/ops18/pc3/y | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B37:anonymous(amount)/ops18/pc5/z | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B38:anonymous(other)/ops18/pc2/x | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B38:anonymous(other)/ops18/pc5/y | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B38:anonymous(other)/ops18/pc8/z | 4008/4005/3/0 | 104596 | hot-SharedSlot:4005@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/4008; depth={'0': 4008}; events={'update': 2} |
| B6:Simulator(numAircraft)/ops37/pc7/push | 1000/0/1000/1000 | 96433 | primitive-prototype:1000@216.5/120.0i | {'eligibility-no-hot-probe': 1000}; {} | 0/0/0/0; depth={}; events={'update': 1} |
| B31:root(other)/ops267/pc0/posOne | 3505/3502/3/0 | 91460 | hot-SharedSlot:3502@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/3005; depth={'0': 3505}; events={'update': 2} |
| B31:root(other)/ops267/pc1/posOne | 3505/3502/3/0 | 91460 | hot-SharedSlot:3502@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/3005; depth={'0': 3505}; events={'update': 2} |
| B13:anonymous(y)/ops53/pc7/right | 3344/3341/3/0 | 87255 | hot-SharedSlot:3341@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2394; depth={'0': 3344}; events={'update': 2} |
| B36:anonymous()/ops12/pc3/squaredMagnitude | 167/0/167/167 | 61530 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:166@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 165, 'receiver-proof-absent': 1}; {'hot-holder-matches': 166, 'receiver-proof-invalid': 166, 'valid-holder-already-cached': 166} | 1/1/1/167; depth={'1': 167}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc185/minus | 167/0/167/167 | 61530 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:166@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 165, 'receiver-proof-absent': 1}; {'hot-holder-matches': 166, 'receiver-proof-invalid': 166, 'valid-holder-already-cached': 166} | 1/1/1/167; depth={'1': 167}; events={'update_from_prototype': 1} |
| B31:root(other)/ops267/pc189/magnitude | 167/0/167/167 | 61530 | general-prototype-chain:1@437.5/69.1i; prototype-own-miss:166@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-identity-mismatch': 165, 'receiver-proof-absent': 1}; {'hot-holder-matches': 166, 'receiver-proof-invalid': 166, 'valid-holder-already-cached': 166} | 1/1/1/167; depth={'1': 167}; events={'update_from_prototype': 1} |
| B13:anonymous(y)/ops53/pc27/parent | 2185/2182/3/0 | 56986 | hot-SharedSlot:2182@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/893; depth={'0': 2185}; events={'update': 2} |
| B9:root(key,value)/ops194/pc52/parent | 2003/2000/3/0 | 52233 | hot-SharedSlot:2000@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2003; depth={'0': 2003}; events={'update': 2} |
| B9:root(key,value)/ops194/pc57/parent | 2003/2000/3/0 | 52233 | hot-SharedSlot:2000@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2003; depth={'0': 2003}; events={'update': 2} |
| B9:root(key,value)/ops194/pc43/parent | 2003/2000/3/0 | 52233 | hot-SharedSlot:2000@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2003; depth={'0': 2003}; events={'update': 2} |
| B9:root(key,value)/ops194/pc51/parent | 2003/2000/3/0 | 52233 | hot-SharedSlot:2000@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2003; depth={'0': 2003}; events={'update': 2} |
| B9:root(key,value)/ops194/pc56/parent | 2003/2000/3/0 | 52233 | hot-SharedSlot:2000@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/2003; depth={'0': 2003}; events={'update': 2} |
| B31:root(other)/ops267/pc136/x | 1336/1333/3/0 | 34813 | hot-SharedSlot:1333@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1336; depth={'0': 1336}; events={'update': 2} |
| B31:root(other)/ops267/pc142/x | 1336/1333/3/0 | 34813 | hot-SharedSlot:1333@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1336; depth={'0': 1336}; events={'update': 2} |
| B31:root(other)/ops267/pc148/y | 1336/1333/3/0 | 34813 | hot-SharedSlot:1333@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1336; depth={'0': 1336}; events={'update': 2} |
| B31:root(other)/ops267/pc154/y | 1336/1333/3/0 | 34813 | hot-SharedSlot:1333@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1336; depth={'0': 1336}; events={'update': 2} |
| B31:root(other)/ops267/pc160/z | 1336/1333/3/0 | 34813 | hot-SharedSlot:1333@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1336; depth={'0': 1336}; events={'update': 2} |
| B31:root(other)/ops267/pc166/z | 1336/1333/3/0 | 34813 | hot-SharedSlot:1333@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1336; depth={'0': 1336}; events={'update': 2} |
| B8:vm(frame)/ops222/pc173/callsign | 1336/1333/3/0 | 34813 | hot-SharedSlot:1333@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1336; depth={'0': 1336}; events={'update': 2} |
| B8:vm(frame)/ops222/pc174/callsign | 1336/1333/3/0 | 34813 | hot-SharedSlot:1333@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1336; depth={'0': 1336}; events={'update': 2} |
| B9:root(key,value)/ops194/pc126/parent | 1134/1131/3/0 | 29538 | hot-SharedSlot:1131@90.0/63.9i; own-lookup:2@unpriced; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 2}; {} | 1/1/1/1134; depth={'0': 1134}; events={'update': 2} |
| B36:anonymous()/ops12/pc1/sqrt | 167/0/167/167 | 20124 | own-lookup:1@unpriced; probe-own-Exact:166@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 166}; {} | 1/1/1/1; depth={'0': 167}; events={'update': 1} |
| B21:vm(motions)/ops42/pc35/forEach | 18/0/18/18 | 6632 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:17@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 17}; {'hot-holder-matches': 17, 'receiver-proof-invalid': 17, 'valid-holder-already-cached': 17} | 1/1/1/18; depth={'1': 18}; events={'update_from_prototype': 1} |
| B8:vm(frame)/ops222/pc75/forEach | 18/0/18/18 | 6632 | general-prototype-chain:1@437.5/69.1i; vm-prototype-own-miss:17@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 17}; {'hot-holder-matches': 17, 'receiver-proof-invalid': 17, 'valid-holder-already-cached': 17} | 1/1/1/1; depth={'1': 18}; events={'update_from_prototype': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc34/now | 18/0/18/18 | 2579 | own-lookup:1@unpriced; probe-own-LiteralShape:17@215.7/64.0i | {'empty-hot-entry': 1, 'unsupported-hot-LiteralShape': 17}; {} | 1/1/1/1; depth={'0': 18}; events={'update': 1} |
| B8:vm(frame)/ops222/pc73/_state | 18/0/18/0 | 2061 | own-lookup:1@unpriced; probe-own-Exact:17@185.0/63.8i | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 17}; {} | 1/1/1/1; depth={'0': 18}; events={'update': 1} |
| B2:vm()/ops72/pc35/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc25/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc28/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc31/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc38/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc41/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc44/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc50/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc47/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc53/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc56/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc59/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc62/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B2:vm()/ops72/pc65/prototype | 1/0/1/0 | 2029 | general-get:1@2098.2/69.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc62/push | 18/0/18/18 | 1736 | primitive-prototype:18@216.5/120.0i | {'eligibility-no-hot-probe': 18}; {} | 0/0/0/0; depth={}; events={'update': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc24/handleNewFrame | 18/16/2/18 | 1515 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:16@117.6/69.0i; prototype-own-miss:1@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 18}; events={'update_from_prototype': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc25/simulate | 18/16/2/18 | 1515 | general-prototype-chain:1@437.5/69.1i; hot-PrototypeSlot:16@117.6/69.0i; prototype-own-miss:1@437.5/69.1i | {'empty-hot-entry': 1, 'receiver-proof-absent': 1}; {'hot-holder-matches': 1, 'receiver-proof-invalid': 1, 'valid-holder-already-cached': 1} | 1/1/1/1; depth={'1': 18}; events={'update_from_prototype': 1} |
| B14:root(callback)/ops40/pc0/_root | 36/32/4/0 | 957 | hot-SharedSlot:32@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/19; depth={'0': 36}; events={'update': 2} |
| B14:root(callback)/ops40/pc9/_root | 36/32/4/0 | 957 | hot-SharedSlot:32@90.0/63.9i; own-lookup:2@unpriced; probe-own-Exact:1@185.0/63.8i; probe-own-SharedSlot:1@unpriced | {'empty-hot-entry': 1, 'unsupported-hot-Exact': 3}; {} | 1/1/1/19; depth={'0': 36}; events={'update': 2} |
| B5:benchmarkImpl(configuration)/ops262/pc39/length | 18/0/18/0 | 667 | primitive-prototype:18@96.0/58.9i | {'eligibility-no-hot-probe': 18}; {} | 0/0/0/0; depth={}; events={} |
| B1:vm()/ops363/pc314/runIteration | 1/0/1/1 | 368 | general-prototype-chain:1@437.5/69.1i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'1': 1}; events={'update_from_prototype': 1} |
| B1:vm()/ops363/pc332/validate | 1/0/1/0 | 368 | general-prototype-chain:1@437.5/69.1i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'3': 1}; events={'update_from_prototype': 1} |
| B1:vm()/ops363/pc294/prepareForNextIteration | 1/0/1/0 | 368 | general-prototype-chain:1@437.5/69.1i | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'3': 1}; events={'update_from_prototype': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc72/splice | 1/0/1/1 | 96 | primitive-prototype:1@216.5/120.0i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'update': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc147/length | 1/0/1/0 | 37 | primitive-prototype:1@96.0/58.9i | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={} |
| B21:vm(motions)/ops42/pc11/length | 18018/0/18018/0 | 0 | general-get:18018@unpriced | {'eligibility-no-hot-probe': 18018}; {} | 0/0/0/0; depth={}; events={'clear': 18018} |
| B8:vm(frame)/ops222/pc171/push | 1336/0/1336/1336 | 0 | general-get:1336@unpriced | {'eligibility-no-hot-probe': 1336}; {} | 0/0/0/0; depth={}; events={'clear': 1336} |
| B5:benchmarkImpl(configuration)/ops262/pc13/now | 1/0/1/1 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc150/numCollisions | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B8:vm(frame)/ops222/pc51/push | 18000/0/18000/18000 | 0 | general-get:18000@unpriced | {'eligibility-no-hot-probe': 18000}; {} | 0/0/0/0; depth={}; events={'clear': 18000} |
| B1:vm()/ops363/pc123/PROXIMITY_RADIUS | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc0/verbosity | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc1/numAircraft | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc2/numFrames | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc3/expectedCollisions | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B5:benchmarkImpl(configuration)/ops262/pc4/exclude | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B1:vm()/ops363/pc145/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc149/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc155/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc167/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc159/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc163/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc175/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc179/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc183/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc187/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc191/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc195/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc199/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc203/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc211/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc215/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc219/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B3:vm()/ops22/pc5/GOOD_VOXEL_SIZE | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B23:voxelHash(position)/ops64/pc37/y | 1/0/1/0 | 0 | own-lookup:1@unpriced | {'empty-hot-entry': 1}; {} | 1/1/1/1; depth={'0': 1}; events={'update': 1} |
| B1:vm()/ops363/pc236/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc244/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B1:vm()/ops363/pc252/prototype | 1/0/1/0 | 0 | general-get:1@unpriced | {'eligibility-no-hot-probe': 1}; {} | 0/0/0/0; depth={}; events={'clear': 1} |
| B8:vm(frame)/ops222/pc13/length | 18018/0/18018/0 | 0 | general-get:18018@unpriced | {'eligibility-no-hot-probe': 18018}; {} | 0/0/0/0; depth={}; events={'clear': 18018} |
| B8:vm(frame)/ops222/pc85/length | 18/0/18/0 | 0 | general-get:18@unpriced | {'eligibility-no-hot-probe': 18}; {} | 0/0/0/0; depth={}; events={'clear': 18} |
| B8:vm(frame)/ops222/pc120/length | 3523/0/3523/0 | 0 | general-get:3523@unpriced | {'eligibility-no-hot-probe': 3523}; {} | 0/0/0/0; depth={}; events={'clear': 3523} |
| B8:vm(frame)/ops222/pc135/length | 10515/0/10515/0 | 0 | general-get:10515@unpriced | {'eligibility-no-hot-probe': 10515}; {} | 0/0/0/0; depth={}; events={'clear': 10515} |
| B8:vm(frame)/ops222/pc152/length | 10515/0/10515/0 | 0 | general-get:10515@unpriced | {'eligibility-no-hot-probe': 10515}; {} | 0/0/0/0; depth={}; events={'clear': 10515} |

#### Resolved-call sites and admission

| Site | Executions, callee family, admission counts |
| --- | --- |
| B10:pc55:wide:method | {'executions': 34140, 'function:anonymous:ordinary:unbound:bytecode': 34140, 'wide': 34140} |
| B10:pc9:wide:method | {'executions': 541391, 'function:anonymous:ordinary:unbound:bytecode': 541391, 'wide': 541391} |
| B11:pc3:wide:method | {'executions': 596719, 'fallback': 596719, 'function:localeCompare:ordinary:unbound:StringPrototypeLocaleCompare': 596719} |
| B14:pc10:wide:plain | {'executions': 36, 'function:treeMinimum:ordinary:unbound:bytecode': 36, 'wide': 36} |
| B14:pc19:wide:plain | {'executions': 26842, 'function:anonymous:ordinary:unbound:bytecode': 26842, 'wide': 26842} |
| B14:pc23:wide:method | {'executions': 26842, 'function:anonymous:ordinary:unbound:bytecode': 26842, 'wide': 26842} |
| B16:pc3:wide:method | {'executions': 18000, 'function:anonymous:ordinary:unbound:bytecode': 18000, 'wide': 18000} |
| B17:pc3:wide:method | {'executions': 30347, 'function:anonymous:ordinary:unbound:bytecode': 30347, 'wide': 30347} |
| B18:pc8:wide:plain | {'executions': 319297, 'function:compare:ordinary:unbound:bytecode': 319297, 'wide': 319297} |
| B19:pc3:wide:method | {'executions': 319297, 'function:anonymous:ordinary:unbound:bytecode': 319297, 'wide': 319297} |
| B1:pc316:vm:method | {'executions': 1, 'function:runIteration:ordinary:unbound:bytecode': 1, 'vm-direct-or-generic': 1} |
| B20:pc6:wide:plain | {'executions': 13749, 'function:treeMinimum:ordinary:unbound:bytecode': 13749, 'wide': 13749} |
| B21:pc37:vm:method | {'executions': 18, 'function:anonymous:ordinary:unbound:bytecode': 18, 'vm-direct-or-generic': 18} |
| B24:pc15:wide:method | {'executions': 24676, 'function:anonymous:ordinary:unbound:bytecode': 24676, 'wide': 24676} |
| B24:pc24:wide:plain | {'executions': 12347, 'function:putIntoMap:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc29:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc30:wide:plain | {'executions': 12347, 'function:recurse:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc35:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc36:wide:plain | {'executions': 12347, 'function:recurse:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc3:wide:plain | {'executions': 116776, 'function:isInVoxel:ordinary:unbound:bytecode': 116776, 'wide': 116776} |
| B24:pc41:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc42:wide:plain | {'executions': 12347, 'function:recurse:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc47:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc48:wide:plain | {'executions': 12347, 'function:recurse:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc53:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc56:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc57:wide:plain | {'executions': 12347, 'function:recurse:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc62:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc65:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc66:wide:plain | {'executions': 12347, 'function:recurse:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc71:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc74:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc75:wide:plain | {'executions': 12347, 'function:recurse:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc80:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc83:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B24:pc84:wide:plain | {'executions': 12347, 'function:recurse:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B26:pc14:wide:method | {'executions': 8842, 'function:anonymous:ordinary:unbound:bytecode': 8842, 'wide': 8842} |
| B26:pc21:wide:method | {'executions': 12347, 'fallback': 12347, 'function:push:ordinary:unbound:ArrayPrototypePush': 12347} |
| B26:pc3:wide:method | {'executions': 12347, 'function:anonymous:ordinary:unbound:bytecode': 12347, 'wide': 12347} |
| B29:pc13:wide:plain | {'executions': 286600, 'function:compareNumbers:ordinary:unbound:bytecode': 286600, 'numeric': 286600} |
| B29:pc3:wide:plain | {'executions': 298109, 'function:compareNumbers:ordinary:unbound:bytecode': 298109, 'numeric': 298109} |
| B30:pc6:wide:method | {'executions': 3505, 'fallback': 3505, 'function:push:ordinary:unbound:ArrayPrototypePush': 3505} |
| B31:pc117:wide:method | {'executions': 1336, 'function:anonymous:ordinary:unbound:bytecode': 1336, 'wide': 1336} |
| B31:pc118:wide:method | {'executions': 1336, 'function:anonymous:ordinary:unbound:bytecode': 1336, 'wide': 1336} |
| B31:pc125:wide:method | {'executions': 1336, 'function:anonymous:ordinary:unbound:bytecode': 1336, 'wide': 1336} |
| B31:pc126:wide:method | {'executions': 1336, 'function:anonymous:ordinary:unbound:bytecode': 1336, 'wide': 1336} |
| B31:pc131:wide:method | {'executions': 1336, 'function:anonymous:ordinary:unbound:bytecode': 1336, 'wide': 1336} |
| B31:pc134:wide:method | {'executions': 1336, 'function:anonymous:ordinary:unbound:bytecode': 1336, 'wide': 1336} |
| B31:pc16:wide:method | {'executions': 3505, 'function:anonymous:ordinary:unbound:bytecode': 3505, 'wide': 3505} |
| B31:pc188:wide:method | {'executions': 167, 'function:anonymous:ordinary:unbound:bytecode': 167, 'wide': 167} |
| B31:pc18:wide:method | {'executions': 3505, 'function:anonymous:ordinary:unbound:bytecode': 3505, 'wide': 3505} |
| B31:pc190:wide:method | {'executions': 167, 'function:anonymous:ordinary:unbound:bytecode': 167, 'wide': 167} |
| B31:pc26:wide:method | {'executions': 3338, 'function:anonymous:ordinary:unbound:bytecode': 3338, 'wide': 3338} |
| B31:pc31:wide:method | {'executions': 3338, 'function:anonymous:ordinary:unbound:bytecode': 3338, 'wide': 3338} |
| B31:pc32:wide:method | {'executions': 3338, 'function:anonymous:ordinary:unbound:bytecode': 3338, 'wide': 3338} |
| B31:pc40:wide:method | {'executions': 3338, 'function:anonymous:ordinary:unbound:bytecode': 3338, 'wide': 3338} |
| B31:pc42:wide:method | {'executions': 3338, 'function:anonymous:ordinary:unbound:bytecode': 3338, 'wide': 3338} |
| B31:pc4:wide:method | {'executions': 3505, 'function:anonymous:ordinary:unbound:bytecode': 3505, 'wide': 3505} |
| B31:pc61:wide:method | {'executions': 3338, 'fallback': 3338, 'function:sqrt:ordinary:unbound:MathSqrt': 3338} |
| B31:pc71:wide:method | {'executions': 3338, 'fallback': 3338, 'function:sqrt:ordinary:unbound:MathSqrt': 3338} |
| B31:pc8:wide:method | {'executions': 3505, 'function:anonymous:ordinary:unbound:bytecode': 3505, 'wide': 3505} |
| B32:pc3:wide:method | {'executions': 7010, 'function:anonymous:ordinary:unbound:bytecode': 7010, 'wide': 7010} |
| B34:pc3:wide:method | {'executions': 7010, 'function:anonymous:ordinary:unbound:bytecode': 7010, 'wide': 7010} |
| B36:pc4:wide:method | {'executions': 167, 'function:anonymous:ordinary:unbound:bytecode': 167, 'wide': 167} |
| B36:pc5:wide:method | {'executions': 167, 'fallback': 167, 'function:sqrt:ordinary:unbound:MathSqrt': 167} |
| B4:pc7:wide:plain | {'executions': 1, 'function:benchmarkImpl:ordinary:unbound:bytecode': 1, 'wide': 1} |
| B5:pc14:wide:method | {'executions': 1, 'function:now:ordinary:unbound:bytecode': 1, 'numeric': 1} |
| B5:pc29:wide:method | {'executions': 18, 'function:anonymous:ordinary:unbound:bytecode': 18, 'wide': 18} |
| B5:pc30:wide:method | {'executions': 18, 'fallback': 18, 'function:anonymous:ordinary:unbound:bytecode': 18} |
| B5:pc35:wide:method | {'executions': 18, 'function:now:ordinary:unbound:bytecode': 18, 'numeric': 18} |
| B5:pc65:wide:method | {'executions': 18, 'fallback': 18, 'function:push:ordinary:unbound:ArrayPrototypePush': 18} |
| B5:pc77:wide:method | {'executions': 1, 'fallback': 1, 'function:splice:ordinary:unbound:ArrayPrototypeSplice': 1} |
| B6:pc12:wide:method | {'executions': 1000, 'fallback': 1000, 'function:push:ordinary:unbound:ArrayPrototypePush': 1000} |
| B7:pc16:wide:method | {'executions': 9000, 'fallback': 9000, 'function:cos:ordinary:unbound:MathCos': 9000} |
| B7:pc25:wide:method | {'executions': 9000, 'fallback': 9000, 'function:push:ordinary:unbound:ArrayPrototypePush': 9000} |
| B7:pc38:wide:method | {'executions': 9000, 'fallback': 9000, 'function:sin:ordinary:unbound:MathSin': 9000} |
| B7:pc47:wide:method | {'executions': 9000, 'fallback': 9000, 'function:push:ordinary:unbound:ArrayPrototypePush': 9000} |
| B8:pc164:vm:method | {'executions': 3505, 'function:anonymous:ordinary:unbound:bytecode': 3505, 'vm-direct-or-generic': 3505} |
| B8:pc178:vm:method | {'executions': 1336, 'function:push:ordinary:unbound:ArrayPrototypePush': 1336, 'vm-direct-or-generic': 1336} |
| B8:pc27:vm:method | {'executions': 18000, 'function:anonymous:ordinary:unbound:bytecode': 18000, 'vm-direct-or-generic': 18000} |
| B8:pc36:vm:method | {'executions': 18000, 'function:anonymous:ordinary:unbound:bytecode': 18000, 'vm-direct-or-generic': 18000} |
| B8:pc57:vm:method | {'executions': 18000, 'function:push:ordinary:unbound:ArrayPrototypePush': 18000, 'vm-direct-or-generic': 18000} |
| B8:pc77:vm:method | {'executions': 18, 'function:anonymous:ordinary:unbound:bytecode': 18, 'vm-direct-or-generic': 18} |
| B9:pc130:wide:method | {'executions': 1134, 'function:anonymous:ordinary:unbound:bytecode': 1134, 'wide': 1134} |
| B9:pc149:wide:method | {'executions': 16937, 'function:anonymous:ordinary:unbound:bytecode': 16937, 'wide': 16937} |
| B9:pc4:wide:method | {'executions': 69518, 'function:anonymous:ordinary:unbound:bytecode': 69518, 'wide': 69518} |
| B9:pc69:wide:method | {'executions': 7752, 'function:anonymous:ordinary:unbound:bytecode': 7752, 'wide': 7752} |
| B9:pc88:wide:method | {'executions': 7849, 'function:anonymous:ordinary:unbound:bytecode': 7849, 'wide': 7849} |

## A's cdjs instruction regression

The old A fallback calls its OwnSlot-only helper for Exact and LiteralShape,
which immediately return None. The byte-identical retained old A binary is
c270267c2617a53271b09aaab6de69531106ac4a446aaf25f7c04c66e73f0a43.
Measured old-A minus base packet costs: Exact +39.012 instructions,
LiteralShape +39.245, hot prototype +9.883, cold prototype +10.069,
hot SharedSlot+.474; OwnSlot-number−143.943. Even the no-property reference
move/control packet adds 3.886 instructions: there is also real code-generation
work, not merely address placement. The old wide executor grew 176 bytes.

CDjs mix: 522159 Exact successes explain20.37M added instructions; 229039 literal
successes8.99M; 924713 hot-prototype reads9.14M; 395755 wide and 39536 VM prototype
fallbacks4.38M; 7726283 shared hits3.66M; primitive paths~3.0M; OwnSlot answers save
~4.03M. Total~46.0M versus~49.15M observed at+0.58%; ~3.1M remains unlocalized.
The mixture is an estimate across operation packets/timing runs, not an exact
exclusive attribution. It falsifies the claim that cdjs's regression is placement.

The reopen will predecode OwnSlot eligibility in the existing hot-index byte,
retaining the original three hot-entry dispatch cases. Masked index plus OwnSlot
bit replaces the old index-bounds handling. Only an OwnSlot-marked entry calls
its borrowed helper; Exact/Literal misses never call it. Updates, writes, probe
selection and clear must keep that hint consistent; receiver/layout/data-slot
checks remain mandatory. Measure all family deltas before the new screen.

## Reproduction and provenance

All commands were wrapped by lockf /tmp/qjs-lead/host-timing.lock, queued behind
the integration session's formal run, then waited for build processes to finish.
No retained sample overlaps a detected competing build. Dirty pricing pairs
would be discarded and restarted; the logs recorded no retained dirty pair.
The initial profile importer failed with main() argument TypeError; the completed
sample was imported through the CLI without resampling. No runtime diagnostic
code is committed; the complete temporary source is retained below for review.

Base standard binary SHA-256 6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4;
NG f7830186 SHA-256 cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0;
diagnostic build SHA-256 9d93f3c3bb69cbeab4a5e98bc65030f9339d6d9a10343be87f5604958a5495f5.
Fresh exact-base sampler receipt SHA-256 26f55dc4d71d58813623dbab9b2842afa86dc708a118e00461712cc29f26e21c:
1248 main-thread samples, 632 flat in wide::activation::run (50.64%);
get_prop_named 15 flat and probe 6 flat. Inclusive functions overlap; do not add them.
This supplements the exact opcode counters; the profile does not price misses.

### collect.py, SHA-256 3902ee90544f06b29ce927ed7c7b3a98b7f38a4dda52e47285e525566dfc8aa9

```python
import sys,os,json,subprocess,hashlib
from pathlib import Path
from tools.benchmark.measure_lock import wait_for_builds
D=Path(__file__).resolve().parent; M=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust');B=D/'build/release/qjs'
while wait_for_builds(60):print('wait: counters not started',flush=True)
raw=[json.loads(s) for s in (M/'target/comparison/brief01-e54e33eb-20261011/external-raw.jsonl').read_text().splitlines()];allrows={}
for suite,case in [('sunspider-1.0','access-binary-trees'),('jetstream3-js-subset','hash-map'),('jetstream3-js-subset','cdjs')]:
 source=M/f'target/bundles/{suite}--{case}.js';official=next(r for r in raw if r['case_id']==case and r['role']=='candidate' and r['phase']=='measurement');assert hashlib.sha256(source.read_bytes()).hexdigest()==official['bundle_sha256']
 r=subprocess.run([str(B),'--raw',str(source)],env={**os.environ,'QJS_PERF_COUNTERS':'1'},capture_output=True,text=True,timeout=300)
 (D/f'{case}.out').write_text(r.stdout);(D/f'{case}.counts').write_text(r.stderr)
 assert r.returncode==0,(case,r.returncode,r.stderr[-2000:]);assert r.stdout==official['stdout'],(case,r.stdout,official['stdout'])
 rows={'sites':[],'shapes':[],'calls':[],'unscoped':[]}
 for l in r.stderr.splitlines():
  for prefix,key in [('SITE','sites'),('SHAPE','shapes'),('CALL','calls'),('UNSCOPED','unscoped')]:
   if l.startswith(prefix+'{'):rows[key].append(json.loads(l[len(prefix):]));break
 for row in rows['sites']:assert row['executions']==row['hot_hits']+row['hot_misses'],row
 allrows[case]=rows;(D/'sites.json').write_text(json.dumps(allrows,indent=2));print(case,'sites',len(rows['sites']),'reads',sum(s['executions'] for s in rows['sites']),'hot',sum(s['hot_hits'] for s in rows['sites']),'methods',sum(s['method_calls'] for s in rows['sites']),rows['unscoped'],flush=True)
print('diagnostic binary sha256',hashlib.sha256(B.read_bytes()).hexdigest(),flush=True)
```

### pricing/measure.py, SHA-256 800deb2edb5481064aa6a7b83056a3f3c07672472a88214151bdbff9e1cac1cf

```python
import os,sys,json,statistics,subprocess,hashlib
from pathlib import Path
from tools.benchmark.screen import sample,counter_tool
from tools.benchmark.measure_lock import wait_for_builds,busy_processes
D=Path(__file__).resolve().parent;N=200000;M=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
B={'base':M/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','old_a':Path('/Users/qingyingliu/Code/js-engine/quickjs-rust-worktrees/wt-quickjs-rust-own-slot-cache-hot-answers-codex/target/release/qjs'),'ng':M/'third_party/quickjs-ng/build/qjs'}
DIAG=D.parent/'build/release/qjs'
def source(name,k,n):
 func=name.startswith('proto') or name.startswith('grand') or name in ('own_function','function_prototype')
 keys=3 if name.endswith('3') else 6 if name.endswith('6') else 12 if name.endswith('12') else 3
 init='function C(){this.x=1;'+''.join('this.p'+str(i)+'=1;' for i in range(keys-1))+'} function marker(){return 1;} '
 if name.startswith('proto') or name.startswith('grand'):init+='C.prototype.f=marker;'
 if name.startswith('grand'):init+='var parent=C.prototype;C.prototype={g:1};Object.setPrototypeOf(C.prototype,parent);'
 if name=='function_prototype':init+='var a=[marker,marker];'
 elif name=='own_function':init+='function own(){var o=new C();o.f=marker;return o;}var a=[own(),own()];'
 elif name=='literal':init+='function literal(){return {x:1,p:2};}var a=[literal(),literal()];'
 else:init+='var a=[new C(),new C()];'
 if name in ('exact','own_number','own_function','proto_hit','grand_hit'):init+='a[1]=a[0];'
 step=('g=r.call;if(g!==f)throw 1;s=s+1;' if name=='function_prototype' else 'g=r.f;if(g!==f)throw 1;s=s+1;') if func else 's=s+r.x;'
 if name=='move':step='g=f;if(g!==f)throw 1;s=s+1;'
 body='var s=1,g=f;'+step*k+'return s;'
 init+='function kernel(r,f){'+body+'} function run(n,a,f,fn){var s=0;for(var i=0;i<n;i++){var r=a[i&1];'+('r.p0=i;' if name=='own_number' else '')+'s+=fn(r,f);}return s;}'
 return init+'var value=run('+str(n)+',a,'+('Function.prototype.call' if name=='function_prototype' else 'marker')+',kernel);if(value!=='+str(n*(k+1))+')throw value;print("OK");'
NAMES=['exact','literal','shared','own_number','own_function','proto_hit','proto_miss3','proto_miss6','proto_miss12','grand_hit','grand_miss6','function_prototype','move']
for name in NAMES:
 for k in [4,8]:
  (D/f'{name}-{k}.js').write_text(source(name,k,N));(D/f'{name}-{k}-diag.js').write_text(source(name,k,32))
if len(sys.argv)>1 and sys.argv[1]=='generate':sys.exit()
while wait_for_builds(60):print('still waiting',flush=True)
if len(sys.argv)>1 and sys.argv[1]=='diagnose':
 for name in NAMES:
  for k in [4,8]:
   r=subprocess.run([str(DIAG),'--raw',str(D/f'{name}-{k}-diag.js')],env={**os.environ,'QJS_PERF_COUNTERS':'1'},capture_output=True,text=True,timeout=60)
   assert r.returncode==0,(name,k,r.stderr[-500:]);(D/f'{name}-{k}.trace').write_text(r.stdout+r.stderr)
 print('micro diagnostics complete',flush=True);sys.exit()
tool=counter_tool();raw=[]
for name in NAMES:
 for pair in range(5):
  while True:
   while wait_for_builds(60):print('waiting before pair',flush=True)
   row={'name':name,'pair':pair,'samples':{}};dirty=False
   for k in ((4,8) if pair%2==0 else (8,4)):
    row['samples'][str(k)]={}
    for role in (tuple(B) if pair%2==0 else tuple(reversed(B))):
     if busy_processes():dirty=True;break
     r=sample(tool,[str(B[role]),str(D/f'{name}-{k}.js')],120)
     if busy_processes():dirty=True;break
     assert 'OK\n' in r.stdout;row['samples'][str(k)][role]={m:getattr(r,m) for m in ['instructions','cycles','wall_ns']}
    if dirty:break
   if not dirty:break
   print('discard contaminated pair',name,pair,flush=True)
  raw.append(row);(D/'raw.json').write_text(json.dumps(raw,indent=2))
 print('measured',name,flush=True)
prices={name:{role:{metric:statistics.median((r['samples']['8'][role][metric]-r['samples']['4'][role][metric])/(4*N) for r in raw if r['name']==name) for metric in ['instructions','cycles']} for role in B} for name in NAMES}
(D/'prices.json').write_text(json.dumps(prices,indent=2));print(json.dumps(prices,indent=2),flush=True)
```

### pricing-extra/measure.py, SHA-256 5999d792760ffa19ce615d7eecf6590c9253f986013ff52c7e27ad49801f1b06

```python
import os,sys,json,statistics,subprocess,hashlib
from pathlib import Path
from tools.benchmark.screen import sample,counter_tool
from tools.benchmark.measure_lock import wait_for_builds,busy_processes
D=Path(__file__).resolve().parent;N=200000;M=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
B={'base':M/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','old_a':Path('/Users/qingyingliu/Code/js-engine/quickjs-rust-worktrees/wt-quickjs-rust-own-slot-cache-hot-answers-codex/target/release/qjs'),'ng':M/'third_party/quickjs-ng/build/qjs'}
DIAG=D.parent/'build/release/qjs'
def source(name,k,n):
 func=name=='string_proto'
 keys=3 if name.endswith('3') else 6 if name.endswith('6') else 12 if name.endswith('12') else 3
 init='function C(){this.x=1;'+''.join('this.p'+str(i)+'=1;' for i in range(keys-1))+'} function marker(){return 1;} '
 if name.startswith('proto') or name.startswith('grand'):init+='C.prototype.f=marker;'
 if name.startswith('grand'):init+='var parent=C.prototype;C.prototype={g:1};Object.setPrototypeOf(C.prototype,parent);'
 if name=='string_proto':init+='var a=["alpha","beta"];'
 elif name=='array_length':init+='var a=[[1],[2]];'
 elif name=='getter':init='function C(){this.p0=1;this.p1=1;this.p2=1;} function marker(){return 1;} Object.defineProperty(C.prototype,"x",{get:function(){return this.p0;}});var a=[new C(),new C()];'
 elif name=='function_prototype':init+='var a=[marker,marker];'
 elif name=='own_function':init+='function own(){var o=new C();o.f=marker;return o;}var a=[own(),own()];'
 elif name=='literal':init+='function literal(){return {x:1,p:2};}var a=[literal(),literal()];'
 else:init+='var a=[new C(),new C()];'
 if name in ('exact','own_number','own_function','proto_hit','grand_hit'):init+='a[1]=a[0];'
 step=('g=r.localeCompare;if(g!==f)throw 1;s=s+1;' if name=='string_proto' else 'g=r.f;if(g!==f)throw 1;s=s+1;') if func else 's=s+r.x;'
 if name=='array_length':step='s=s+r.length;'
 if name=='move':step='g=f;if(g!==f)throw 1;s=s+1;'
 body='var s=1,g=f;'+step*k+'return s;'
 init+='function kernel(r,f){'+body+'} function run(n,a,f,fn){var s=0;for(var i=0;i<n;i++){var r=a[i&1];'+('r.p0=i;' if name=='own_number' else '')+'s+=fn(r,f);}return s;}'
 return init+'var value=run('+str(n)+',a,'+('String.prototype.localeCompare' if name=='string_proto' else 'marker')+',kernel);if(value!=='+str(n*(k+1))+')throw value;print("OK");'
NAMES=['string_proto','array_length','getter']
for name in NAMES:
 for k in [4,8]:
  (D/f'{name}-{k}.js').write_text(source(name,k,N));(D/f'{name}-{k}-diag.js').write_text(source(name,k,32))
if len(sys.argv)>1 and sys.argv[1]=='generate':sys.exit()
while wait_for_builds(60):print('still waiting',flush=True)
if len(sys.argv)>1 and sys.argv[1]=='diagnose':
 for name in NAMES:
  for k in [4,8]:
   r=subprocess.run([str(DIAG),'--raw',str(D/f'{name}-{k}-diag.js')],env={**os.environ,'QJS_PERF_COUNTERS':'1'},capture_output=True,text=True,timeout=60)
   assert r.returncode==0,(name,k,r.stderr[-500:]);(D/f'{name}-{k}.trace').write_text(r.stdout+r.stderr)
 print('micro diagnostics complete',flush=True);sys.exit()
tool=counter_tool();raw=[]
for name in NAMES:
 for pair in range(5):
  while True:
   while wait_for_builds(60):print('waiting before pair',flush=True)
   row={'name':name,'pair':pair,'samples':{}};dirty=False
   for k in ((4,8) if pair%2==0 else (8,4)):
    row['samples'][str(k)]={}
    for role in (tuple(B) if pair%2==0 else tuple(reversed(B))):
     if busy_processes():dirty=True;break
     r=sample(tool,[str(B[role]),str(D/f'{name}-{k}.js')],120)
     if busy_processes():dirty=True;break
     assert 'OK\n' in r.stdout;row['samples'][str(k)][role]={m:getattr(r,m) for m in ['instructions','cycles','wall_ns']}
    if dirty:break
   if not dirty:break
   print('discard contaminated pair',name,pair,flush=True)
  raw.append(row);(D/'raw.json').write_text(json.dumps(raw,indent=2))
 print('measured',name,flush=True)
prices={name:{role:{metric:statistics.median((r['samples']['8'][role][metric]-r['samples']['4'][role][metric])/(4*N) for r in raw if r['name']==name) for metric in ['instructions','cycles']} for role in B} for name in NAMES}
(D/'prices.json').write_text(json.dumps(prices,indent=2));print(json.dumps(prices,indent=2),flush=True)
```

### pricing-reference/measure.py, SHA-256 1342fcf800a009ba2a2b59272843543ca9d7b20546eeb02938b0be02f6fb1ae6

```python
import os,sys,json,statistics,subprocess,hashlib
from pathlib import Path
from tools.benchmark.screen import sample,counter_tool
from tools.benchmark.measure_lock import wait_for_builds,busy_processes
D=Path(__file__).resolve().parent;N=200000;M=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
B={'base':M/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','old_a':Path('/Users/qingyingliu/Code/js-engine/quickjs-rust-worktrees/wt-quickjs-rust-own-slot-cache-hot-answers-codex/target/release/qjs'),'ng':M/'third_party/quickjs-ng/build/qjs'}
DIAG=D.parent/'build/release/qjs'
def source(name,k,n):
 arr=name.endswith('array');value='[1]' if arr else '{a:1}'
 step='g=v;if(g!==v)throw 1;s=s+1;' if name.startswith('move') else 'g=r.x;if(g!==v)throw 1;s=s+1;'
 init='var v='+value+';function C(){this.x=v;this.p0=0;this.p1=1;} var r=new C();'
 body='var s=1,g=v;'+step*k+'return s;'
 return init+'function kernel(r,v){'+body+'}function run(n,r,v,fn){var s=0;for(var i=0;i<n;i++){r.p0=i;s+=fn(r,v);}return s;}var result=run('+str(n)+',r,v,kernel);if(result!=='+str(n*(k+1))+')throw result;print("OK");'

NAMES=['own_object','own_array','move_object','move_array']
for name in NAMES:
 for k in [4,8]:
  (D/f'{name}-{k}.js').write_text(source(name,k,N));(D/f'{name}-{k}-diag.js').write_text(source(name,k,32))
if len(sys.argv)>1 and sys.argv[1]=='generate':sys.exit()
while wait_for_builds(60):print('still waiting',flush=True)
if len(sys.argv)>1 and sys.argv[1]=='diagnose':
 for name in NAMES:
  for k in [4,8]:
   r=subprocess.run([str(DIAG),'--raw',str(D/f'{name}-{k}-diag.js')],env={**os.environ,'QJS_PERF_COUNTERS':'1'},capture_output=True,text=True,timeout=60)
   assert r.returncode==0,(name,k,r.stderr[-500:]);(D/f'{name}-{k}.trace').write_text(r.stdout+r.stderr)
 print('micro diagnostics complete',flush=True);sys.exit()
tool=counter_tool();raw=[]
for name in NAMES:
 for pair in range(5):
  while True:
   while wait_for_builds(60):print('waiting before pair',flush=True)
   row={'name':name,'pair':pair,'samples':{}};dirty=False
   for k in ((4,8) if pair%2==0 else (8,4)):
    row['samples'][str(k)]={}
    for role in (tuple(B) if pair%2==0 else tuple(reversed(B))):
     if busy_processes():dirty=True;break
     r=sample(tool,[str(B[role]),str(D/f'{name}-{k}.js')],120)
     if busy_processes():dirty=True;break
     assert 'OK\n' in r.stdout;row['samples'][str(k)][role]={m:getattr(r,m) for m in ['instructions','cycles','wall_ns']}
    if dirty:break
   if not dirty:break
   print('discard contaminated pair',name,pair,flush=True)
  raw.append(row);(D/'raw.json').write_text(json.dumps(raw,indent=2))
 print('measured',name,flush=True)
prices={name:{role:{metric:statistics.median((r['samples']['8'][role][metric]-r['samples']['4'][role][metric])/(4*N) for r in raw if r['name']==name) for metric in ['instructions','cycles']} for role in B} for name in NAMES}
(D/'prices.json').write_text(json.dumps(prices,indent=2));print(json.dumps(prices,indent=2),flush=True)
```

### pricing-numeric/measure.py, SHA-256 ce6bf9a46e4c22433b5b135d833d0913c44f917c4d3969e152990ebb1595b81c

```python
import os,sys,json,statistics,subprocess,hashlib
from pathlib import Path
from tools.benchmark.screen import sample,counter_tool
from tools.benchmark.measure_lock import wait_for_builds,busy_processes
D=Path(__file__).resolve().parent;N=200000;M=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
B={'base':M/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','old_a':Path('/Users/qingyingliu/Code/js-engine/quickjs-rust-worktrees/wt-quickjs-rust-own-slot-cache-hot-answers-codex/target/release/qjs'),'ng':M/'third_party/quickjs-ng/build/qjs'}
DIAG=D.parent/'build/release/qjs'
def source(name,k,n):
 func=False
 keys=3 if name.endswith('3') else 6 if name.endswith('6') else 12 if name.endswith('12') else 3
 init='function C(){this.x=1;'+''.join('this.p'+str(i)+'=1;' for i in range(keys-1))+'} function marker(){return 1;} '
 if name.startswith('proto') or name.startswith('grand'):init+='C.prototype.f=marker;'
 if name.startswith('grand'):init+='var parent=C.prototype;C.prototype={g:1};Object.setPrototypeOf(C.prototype,parent);'
 if name=='string_proto':init+='var a=["alpha","beta"];'
 elif name=='array_length':init+='var a=[[1],[2]];'
 elif name=='getter':init='function C(){this.p0=1;this.p1=1;this.p2=1;} function marker(){return 1;} Object.defineProperty(C.prototype,"x",{get:function(){return this.p0;}});var a=[new C(),new C()];'
 elif name=='function_prototype':init+='var a=[marker,marker];'
 elif name=='own_function':init+='function own(){var o=new C();o.f=marker;return o;}var a=[own(),own()];'
 elif name=='literal':init+='function literal(){return {x:1,p:2};}var a=[literal(),literal()];'
 else:init+='var a=[new C(),new C()];'
 if name in ('exact','own_number','own_function','proto_hit','grand_hit'):init+='a[1]=a[0];'
 step=('g=r.localeCompare;if(g!==f)throw 1;s=s+1;' if name=='string_proto' else 'g=r.f;if(g!==f)throw 1;s=s+1;') if func else 's=s+v;'
 if name=='array_length':step='s=s+r.length;'
 if name=='move':step='g=f;if(g!==f)throw 1;s=s+1;'
 body='var s=1;if(r===null)throw 1;'+step*k+'return s;'
 init+='function kernel(r,v){'+body+'} function run(n,a,f,fn){var s=0;for(var i=0;i<n;i++){var r=a[i&1];'+('r.p0=i;' if name=='own_number' else '')+'s+=fn(r,f);}return s;}'
 return init+'var value=run('+str(n)+',a,'+'1'+',kernel);if(value!=='+str(n*(k+1))+')throw value;print("OK");'
NAMES=['numeric_move']
for name in NAMES:
 for k in [4,8]:
  (D/f'{name}-{k}.js').write_text(source(name,k,N));(D/f'{name}-{k}-diag.js').write_text(source(name,k,32))
if len(sys.argv)>1 and sys.argv[1]=='generate':sys.exit()
while wait_for_builds(60):print('still waiting',flush=True)
if len(sys.argv)>1 and sys.argv[1]=='diagnose':
 for name in NAMES:
  for k in [4,8]:
   r=subprocess.run([str(DIAG),'--raw',str(D/f'{name}-{k}-diag.js')],env={**os.environ,'QJS_PERF_COUNTERS':'1'},capture_output=True,text=True,timeout=60)
   assert r.returncode==0,(name,k,r.stderr[-500:]);(D/f'{name}-{k}.trace').write_text(r.stdout+r.stderr)
 print('micro diagnostics complete',flush=True);sys.exit()
tool=counter_tool();raw=[]
for name in NAMES:
 for pair in range(5):
  while True:
   while wait_for_builds(60):print('waiting before pair',flush=True)
   row={'name':name,'pair':pair,'samples':{}};dirty=False
   for k in ((4,8) if pair%2==0 else (8,4)):
    row['samples'][str(k)]={}
    for role in (tuple(B) if pair%2==0 else tuple(reversed(B))):
     if busy_processes():dirty=True;break
     r=sample(tool,[str(B[role]),str(D/f'{name}-{k}.js')],120)
     if busy_processes():dirty=True;break
     assert 'OK\n' in r.stdout;row['samples'][str(k)][role]={m:getattr(r,m) for m in ['instructions','cycles','wall_ns']}
    if dirty:break
   if not dirty:break
   print('discard contaminated pair',name,pair,flush=True)
  raw.append(row);(D/'raw.json').write_text(json.dumps(raw,indent=2))
 print('measured',name,flush=True)
prices={name:{role:{metric:statistics.median((r['samples']['8'][role][metric]-r['samples']['4'][role][metric])/(4*N) for r in raw if r['name']==name) for metric in ['instructions','cycles']} for role in B} for name in NAMES}
(D/'prices.json').write_text(json.dumps(prices,indent=2));print(json.dumps(prices,indent=2),flush=True)
```

### Temporary instrumentation patch

```diff
diff --git a/crates/qjs-runtime/src/bytecode/compact_fn/property.rs b/crates/qjs-runtime/src/bytecode/compact_fn/property.rs
index 4d293e7c..d82d49b5 100644
--- a/crates/qjs-runtime/src/bytecode/compact_fn/property.rs
+++ b/crates/qjs-runtime/src/bytecode/compact_fn/property.rs
@@ -60,10 +60,14 @@ pub(super) fn get_prop_named(
             && cache.receiver_miss_proven(object_ref)
             && let Some(value) = holder.prototype_data_slot_value(*slot)
         {
+            #[cfg(feature = "perf-counters")]
+            crate::bytecode::named_property_cache::site_diagnostics::answer("prototype-proven-receiver");
             return Ok(value);
         }
         match object_ref.own_data_property_read(key) {
             OwnDataPropertyRead::Data(value) => {
+                #[cfg(feature = "perf-counters")]
+                crate::bytecode::named_property_cache::site_diagnostics::answer("own-lookup");
                 cache.update(object_ref, key, &value);
                 return Ok(value);
             }
@@ -74,6 +78,8 @@ pub(super) fn get_prop_named(
                 if let CacheProbe::PrototypeCandidate { holder, slot } = probe
                     && let Some(value) = holder.prototype_data_slot_value(slot)
                 {
+                    #[cfg(feature = "perf-counters")]
+                    crate::bytecode::named_property_cache::site_diagnostics::answer("prototype-own-miss");
                     cache.remember_receiver_miss(object_ref);
                     return Ok(value);
                 }
@@ -86,8 +92,12 @@ pub(super) fn get_prop_named(
                     #[cfg(feature = "agents")]
                     env.agent_context(),
                 ) {
+                    #[cfg(feature = "perf-counters")]
+                    crate::bytecode::named_property_cache::site_diagnostics::answer("direct-getter");
                     return result;
                 }
+                #[cfg(feature = "perf-counters")]
+                crate::bytecode::named_property_cache::site_diagnostics::answer("general-prototype-chain");
                 let mut call_env = env.empty_frame();
                 let value =
                     crate::bytecode::vm_props::get_property(object.clone(), key, &mut call_env)?;
@@ -98,9 +108,13 @@ pub(super) fn get_prop_named(
         }
     }
     if let Some(value) = prototype_receiver_named_value(object, key, cache, env) {
+        #[cfg(feature = "perf-counters")]
+        crate::bytecode::named_property_cache::site_diagnostics::answer("primitive-prototype");
         return Ok(value);
     }
     cache.clear();
+    #[cfg(feature = "perf-counters")]
+    crate::bytecode::named_property_cache::site_diagnostics::answer("general-get");
     let mut call_env = env.empty_frame();
     crate::bytecode::vm_props::get_property(object.clone(), key, &mut call_env)
 }
diff --git a/crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs b/crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs
index e20c1c34..a2e6122b 100644
--- a/crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs
+++ b/crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs
@@ -818,12 +818,22 @@ fn run_frames(
                         ) else {
                             break Err(execute::uninitialized_local());
                         };
+                        #[cfg(feature = "perf-counters")]
+                        let _site = crate::bytecode::named_property_cache::site_diagnostics::begin(
+                            &site.cache, bytecode, match &current_callee { Value::Function(f) => f.name.as_deref().unwrap_or("anonymous"), _ => root.upvalues.function().and_then(|f| f.name.as_deref()).unwrap_or("root") },
+                            pc - 1, &site.key, "wide", (current_base, dst as usize), Some(receiver));
                         if let Some(value) = property::get_prop_named_hot(receiver, &site.cache) {
+                            #[cfg(feature = "perf-counters")]
+                            _site.finish(Some(&value));
                             execute::store(&mut window[dst as usize], value);
                             continue;
                         }
                         match property::get_prop_named(receiver, &site.key, &site.cache, env) {
-                            Ok(value) => execute::store(&mut window[dst as usize], value),
+                            Ok(value) => {
+                                #[cfg(feature = "perf-counters")]
+                                _site.finish(Some(&value));
+                                execute::store(&mut window[dst as usize], value);
+                            },
                             Err(error) => break Err(error),
                         }
                     }
@@ -833,9 +843,15 @@ fn run_frames(
                         };
                         // The receiver is borrowed: a fused site peeks its
                         // local, and the plain form's result replaces it.
+                        #[cfg(feature = "perf-counters")]
+                        let _site = crate::bytecode::named_property_cache::site_diagnostics::begin(
+                            &site.cache, bytecode, match &current_callee { Value::Function(f) => f.name.as_deref().unwrap_or("anonymous"), _ => root.upvalues.function().and_then(|f| f.name.as_deref()).unwrap_or("root") },
+                            pc - 1, &site.key, "wide", (current_base, dst as usize), Some(&window[obj as usize]));
                         if let Some(value) =
                             property::get_prop_named_hot(&window[obj as usize], &site.cache)
                         {
+                            #[cfg(feature = "perf-counters")]
+                            _site.finish(Some(&value));
                             execute::store(&mut window[dst as usize], value);
                             continue;
                         }
@@ -845,7 +861,11 @@ fn run_frames(
                             &site.cache,
                             env,
                         ) {
-                            Ok(value) => execute::store(&mut window[dst as usize], value),
+                            Ok(value) => {
+                                #[cfg(feature = "perf-counters")]
+                                _site.finish(Some(&value));
+                                execute::store(&mut window[dst as usize], value);
+                            },
                             Err(error) => break Err(error),
                         }
                     }
@@ -1293,6 +1313,10 @@ fn run_frames(
         let argument_index = callee_index + 1;
         let argument_count = argc as usize;
 
+        #[cfg(feature = "perf-counters")]
+        let _call = crate::bytecode::named_property_cache::site_diagnostics::call_begin(
+            running_bytecode(&current_callee, root_bytecode), resume_pc - 1,
+            "wide", (current_base, base as usize), &callee, resolved);
         if let Some(inline) = inline_callee(&callee, env) {
             // A numeric body runs its whole call tree on `f64` registers,
             // leaving the arguments where they are: they are numbers.
@@ -1302,6 +1326,8 @@ fn run_frames(
                     &registers[argument_index..argument_index + argument_count],
                 )
             {
+                #[cfg(feature = "perf-counters")]
+                crate::bytecode::named_property_cache::site_diagnostics::call_outcome(&_call, "numeric");
                 if let Some(receiver) = receiver {
                     release(receiver);
                 }
@@ -1313,6 +1339,8 @@ fn run_frames(
                 unwind(registers, frames, current_base + current_len);
                 return Err(call_stack_exhausted());
             }
+            #[cfg(feature = "perf-counters")]
+            crate::bytecode::named_property_cache::site_diagnostics::call_outcome(&_call, "wide");
             crate::diagnostics::count!(ordinary_call_attempts);
             crate::diagnostics::count!(compact_direct_calls);
             let callee_base = current_base + current_len;
@@ -1381,6 +1409,8 @@ fn run_frames(
             continue;
         }
 
+        #[cfg(feature = "perf-counters")]
+        crate::bytecode::named_property_cache::site_diagnostics::call_outcome(&_call, "fallback");
         let outcome = {
             let bytecode = running_bytecode(&current_callee, root_bytecode);
             let (upvalue_owner, upvalue_slots) = match &current_callee {
diff --git a/crates/qjs-runtime/src/bytecode/named_property_cache.rs b/crates/qjs-runtime/src/bytecode/named_property_cache.rs
index a33e609e..3cbfc03f 100644
--- a/crates/qjs-runtime/src/bytecode/named_property_cache.rs
+++ b/crates/qjs-runtime/src/bytecode/named_property_cache.rs
@@ -23,6 +23,8 @@ const POLYMORPHIC_CACHE_SLOTS: usize = 4;
 
 #[cfg(test)]
 mod tests;
+#[cfg(feature = "perf-counters")]
+pub(in crate::bytecode) mod site_diagnostics;
 
 #[derive(Clone, Debug, Default)]
 pub(super) struct NamedPropertyCache(Rc<RefCell<NamedPropertyCacheState>>);
@@ -167,6 +169,14 @@ impl NamedPropertyCache {
     /// same proof `get_prop_named` requires of a probe's candidate.
     #[inline(always)]
     pub(super) fn probe_hot(&self, object: &ObjectRef) -> Option<Value> {
+        let result = self.probe_hot_inner(object);
+        #[cfg(feature = "perf-counters")]
+        site_diagnostics::hot(self, object, result.as_ref());
+        result
+    }
+
+    #[inline(always)]
+    fn probe_hot_inner(&self, object: &ObjectRef) -> Option<Value> {
         let state = self.0.borrow();
         match state.entries.get(usize::from(state.hot.get()))? {
             Some(NamedPropertyCacheEntry::SharedSlot { key, slot }) => {
@@ -212,6 +222,13 @@ impl NamedPropertyCache {
     }
 
     pub(super) fn probe(&self, object: &ObjectRef) -> CacheProbe {
+        let result = self.probe_inner(object);
+        #[cfg(feature = "perf-counters")]
+        site_diagnostics::probe(self, &result);
+        result
+    }
+
+    fn probe_inner(&self, object: &ObjectRef) -> CacheProbe {
         let state = self.0.borrow();
         let mut candidate = None;
         // Unrolled by hand: whether LLVM unrolled the walk varied from build
@@ -347,6 +364,8 @@ impl NamedPropertyCache {
     /// non-ordinary prototype, or storage without stable slots -- installs
     /// nothing, so the read stays on the general path.
     pub(super) fn update_from_prototype(&self, receiver: &ObjectRef, key: &str) {
+        #[cfg(feature = "perf-counters")]
+        site_diagnostics::cache_event(self, "update_from_prototype");
         let Some(parent) = receiver.ordinary_prototype() else {
             return;
         };
@@ -408,6 +427,8 @@ impl NamedPropertyCache {
     }
 
     pub(super) fn update(&self, object: &ObjectRef, key: &str, value: &Value) {
+        #[cfg(feature = "perf-counters")]
+        site_diagnostics::cache_event(self, "update");
         #[cfg(feature = "perf-counters")]
         if std::env::var_os("QJS_IC_TRACE").is_some() {
             eprintln!(
@@ -569,6 +590,8 @@ impl NamedPropertyCache {
     }
 
     pub(super) fn record_write(&self, object: &ObjectRef, key: &str) {
+        #[cfg(feature = "perf-counters")]
+        site_diagnostics::cache_event(self, "record_write");
         let mut state = self.0.borrow_mut();
         let saw_other_object = state.entries.iter().flatten().any(|entry| {
             matches!(
@@ -600,6 +623,8 @@ impl NamedPropertyCache {
     }
 
     pub(super) fn clear(&self) {
+        #[cfg(feature = "perf-counters")]
+        site_diagnostics::cache_event(self, "clear");
         let mut state = self.0.borrow_mut();
         state.entries = Default::default();
         state.next_slot = 0;
diff --git a/crates/qjs-runtime/src/bytecode/vm/general_ops.rs b/crates/qjs-runtime/src/bytecode/vm/general_ops.rs
index 605cd011..af33f26c 100644
--- a/crates/qjs-runtime/src/bytecode/vm/general_ops.rs
+++ b/crates/qjs-runtime/src/bytecode/vm/general_ops.rs
@@ -319,9 +319,16 @@ impl Vm<'_> {
         key: &Rc<str>,
         cache: &NamedPropertyCache,
     ) -> Result<(), RuntimeError> {
+        #[cfg(feature = "perf-counters")]
+        let _site = crate::bytecode::named_property_cache::site_diagnostics::begin(
+            cache, &self.bytecode, "vm", self.ip.saturating_sub(1), key, "vm",
+            (self as *const Self as usize, self.stack.len().saturating_sub(usize::from(cache.local_slot().is_none()))),
+            cache.local_slot().and_then(|s| self.locals.get(s)).and_then(|v|v.as_ref()).or_else(||self.stack.last()));
         crate::diagnostics::count!(named_property_reads);
         let result = self.get_named_prop(key, cache);
         self.handle_runtime_result(result)?;
+        #[cfg(feature = "perf-counters")]
+        _site.finish(self.stack.last());
         Ok(())
     }
 
diff --git a/crates/qjs-runtime/src/bytecode/vm_call.rs b/crates/qjs-runtime/src/bytecode/vm_call.rs
index bb408fd6..f216c88c 100644
--- a/crates/qjs-runtime/src/bytecode/vm_call.rs
+++ b/crates/qjs-runtime/src/bytecode/vm_call.rs
@@ -460,6 +460,11 @@ impl Vm<'_> {
     /// Calls a pre-resolved callee whose receiver and callee are already on the
     /// stack as `[receiver, callee, args...]`.
     pub(super) fn call_resolved(&mut self, argc: usize) -> Result<(), RuntimeError> {
+        #[cfg(feature = "perf-counters")]
+        let _call = self.stack.len().checked_sub(argc + 1).map(|i| crate::bytecode::named_property_cache::site_diagnostics::call_begin(
+            &self.bytecode, self.ip.saturating_sub(1), "vm", (self as *const Self as usize, i), &self.stack[i], true));
+        #[cfg(feature = "perf-counters")]
+        if let Some(c) = &_call { crate::bytecode::named_property_cache::site_diagnostics::call_outcome(c,"vm-direct-or-generic"); }
         let direct_leaf = argc <= 1
             && self
                 .stack
```

### Temporary collector module

```rust
//! Temporary e54 per-site counters. This module is never timed or committed.
use std::{cell::RefCell, collections::{BTreeMap,HashMap,HashSet},rc::Rc};
use super::{NamedPropertyCache,NamedPropertyCacheEntry,CacheProbe};
use crate::{Value,ObjectRef};
use crate::value::ObjectWeakRef;
use crate::bytecode::ir::Bytecode;

#[derive(Default)]
struct Site {label:String,key:String, executions:u64,hot_hits:u64,hot_misses:u64,repeated_hot:u64, reasons:BTreeMap<String,u64>, guards:BTreeMap<String,u64>, answers:BTreeMap<String,u64>, types:BTreeMap<String,u64>,inputs:BTreeMap<String,u64>,tiers:BTreeMap<String,u64>, shapes:BTreeMap<usize,u64>, unordered:HashSet<usize>,shared:HashSet<usize>, literal:HashSet<usize>, epochs:BTreeMap<u64,u64>, receivers:HashSet<usize>, depths:BTreeMap<usize,u64>, cache_events:BTreeMap<String,u64>,method_calls:u64,call_outcomes:BTreeMap<String,u64>, cache:Option<NamedPropertyCache>}
struct Active {id:usize,first_hot:bool,observed:bool,answer:String,dest:(String,usize,usize),ended:bool}
struct Snapshot {weak:ObjectWeakRef,epoch:u64,proto:usize,ordered:usize,unordered:usize,shared:usize,literal:usize}
#[derive(Default)]
struct Store {sites:BTreeMap<usize,Site>,active:Vec<Active>,bodies:HashMap<usize,usize>,shapes:Vec<String>,shape_ids:HashMap<String,usize>,objects:HashMap<usize,Snapshot>,registers:HashMap<(String,usize,usize),usize>,calls:BTreeMap<String,BTreeMap<String,u64>>,unscoped:u64}
thread_local! {static STORE:RefCell<Store>=RefCell::new(Store::default());}
fn inc(m:&mut BTreeMap<String,u64>,k:&str){*m.entry(k.into()).or_default()+=1;}
fn j(s:&str)->String {format!("{s:?}")}
fn map(m:&BTreeMap<String,u64>)->String{format!("{{{}}}",m.iter().map(|(k,v)|format!("{}:{v}",j(k))).collect::<Vec<_>>().join(","))}
fn counts<K:std::fmt::Display+Ord>(m:&BTreeMap<K,u64>)->String{format!("{{{}}}",m.iter().map(|(k,v)|format!("{}:{v}",j(&k.to_string()))).collect::<Vec<_>>().join(","))}
fn typ(v:&Value)->String {match v {Value::Function(f)=>format!("function:{}:{}:{}:{}",f.name.as_deref().unwrap_or("anonymous"),if f.is_class_constructor{"class"}else{"ordinary"},if f.bound.is_some(){"bound"}else{"unbound"},f.native.map(|n|format!("{n:?}")).unwrap_or("bytecode".into())),Value::Object(_)=>"object".into(),Value::Number(_)=>"number".into(),Value::String(_)=>"string".into(),Value::Null=>"null".into(),Value::Array(_)=>"array".into(),_=>"other".into()}}
fn kind(e:&NamedPropertyCacheEntry)->&'static str {match e {NamedPropertyCacheEntry::Exact{..}=>"Exact",NamedPropertyCacheEntry::OwnSlot{..}=>"OwnSlot",NamedPropertyCacheEntry::SharedSlot{..}=>"SharedSlot",NamedPropertyCacheEntry::LiteralShape{..}=>"LiteralShape",NamedPropertyCacheEntry::PrototypeSlot{..}=>"PrototypeSlot",NamedPropertyCacheEntry::GrandprototypeSlot{..}=>"GrandprototypeSlot"}}
impl Store {
 fn intern(&mut self,s:String)->usize {if let Some(i)=self.shape_ids.get(&s){return *i;} let id=self.shapes.len();self.shapes.push(s.clone());self.shape_ids.insert(s,id);id}
 fn body(&mut self,b:&Bytecode)->usize {let next=self.bodies.len()+1;*self.bodies.entry(b as *const Bytecode as usize).or_insert(next)}
 fn observe(&mut self,id:usize,o:&ObjectRef,key:&str){
  let addr=o.address();let epoch=o.layout_revision();let proto=o.ordinary_prototype().map(|p|p.address()).unwrap_or(0);
  if !self.objects.get(&addr).is_some_and(|s|s.weak.ptr_eq(o)&&s.epoch==epoch&&s.proto==proto){
   let names=if o.string_data().is_some(){Vec::new()}else{o.own_string_keys_with_enumerability()};let mut fields=Vec::new();let mut shared=Vec::new();let mut literal=0;
   for (k,_) in &names {let p=o.property(k);fields.push(format!("{}:{}:{}:{}:{}",k,p.as_ref().is_some_and(|p|p.is_accessor()),p.as_ref().is_some_and(|p|p.enumerable),p.as_ref().is_some_and(|p|p.writable),p.as_ref().is_some_and(|p|p.configurable)));shared.push(format!("{}@{:x}",k,Rc::as_ptr(k) as *const () as usize));if literal==0 {literal=o.literal_data_slot(k).map(|(s,_)|Rc::as_ptr(&s) as usize).unwrap_or(0);}}
   let storage=o.storage_kind_for_trace().to_string();let ordered=self.intern(format!("{storage};{}",fields.join("|")));fields.sort();let unordered=self.intern(format!("{storage};{}",fields.join("|")));let shared=self.intern(format!("{storage};{}",shared.join("|")));
   self.objects.insert(addr,Snapshot{weak:o.downgrade(),epoch,proto,ordered,unordered,shared,literal});
  }
  let snapshot=&self.objects[&addr];let row=self.sites.get_mut(&id).unwrap();*row.shapes.entry(snapshot.ordered).or_default()+=1;row.unordered.insert(snapshot.unordered);row.shared.insert(snapshot.shared);if snapshot.literal!=0{row.literal.insert(snapshot.literal);}*row.epochs.entry(epoch).or_default()+=1;row.receivers.insert(addr);
  let mut depth=0;let mut current=Some(o.clone());while let Some(p)=current {if !matches!(p.own_data_property_read(key),crate::value::OwnDataPropertyRead::Missing){break;}depth+=1;if depth>8{break;}current=p.ordinary_prototype();}*row.depths.entry(depth).or_default()+=1;
 }
}
pub struct Guard(usize);
pub fn begin(cache:&NamedPropertyCache,b:&Bytecode,name:&str,pc:usize,key:&str,tier:&str,dest:(usize,usize),receiver:Option<&Value>)->Guard {
 let id=Rc::as_ptr(&cache.0) as usize;
 STORE.with(|s|{let mut s=s.borrow_mut();let body=s.body(b);let row=s.sites.entry(id).or_default();if row.label.is_empty(){row.label=format!("B{body}:{name}({})/ops{}/pc{pc}/{key}",b.parameter_names().join(","),b.code.len());row.key=key.into();row.cache=Some(cache.clone());}row.executions+=1;inc(&mut row.tiers,tier);if let Some(v)=receiver{inc(&mut row.inputs,&typ(v));}if let Some(Value::Object(o))=receiver{s.observe(id,o,key);}let token=s.active.len();s.active.push(Active{id,first_hot:false,observed:matches!(receiver,Some(Value::Object(_))),answer:"general-get".into(),dest:(tier.into(),dest.0,dest.1),ended:false});Guard(token)})
}
impl Guard {pub fn finish(&self,value:Option<&Value>){STORE.with(|s|{let mut s=s.borrow_mut();let (id,answer,first,dest)={let a=&mut s.active[self.0];if a.ended{return;}a.ended=true;(a.id,a.answer.clone(),a.first_hot,a.dest.clone())};let row=s.sites.get_mut(&id).unwrap();if !first {row.hot_misses+=1;inc(&mut row.reasons,"eligibility-no-hot-probe");}inc(&mut row.answers,&answer);if let Some(v)=value{inc(&mut row.types,&typ(v));s.registers.insert(dest,id);}});}}
impl Drop for Guard {fn drop(&mut self){self.finish(None);STORE.with(|s|{s.borrow_mut().active.pop();});}}
pub fn answer(value:&str){STORE.with(|s|{if let Some(a)=s.borrow_mut().active.last_mut(){a.answer=value.into();}});}
pub fn hot(cache:&NamedPropertyCache,o:&ObjectRef,value:Option<&Value>){
 let id=Rc::as_ptr(&cache.0) as usize;
 let state=cache.0.borrow();let entry=state.entries.get(usize::from(state.hot.get())).and_then(|e|e.as_ref());
 let reason=if value.is_some(){"hit"}else{match entry {
 None=>"empty-hot-entry",
 Some(NamedPropertyCacheEntry::Exact{..})=>"unsupported-hot-Exact",
 Some(NamedPropertyCacheEntry::LiteralShape{..})=>"unsupported-hot-LiteralShape",
 Some(NamedPropertyCacheEntry::OwnSlot{..})=>"unsupported-hot-OwnSlot",
 Some(NamedPropertyCacheEntry::SharedSlot{..})=>"shared-slot-key-or-layout-mismatch",
 Some(NamedPropertyCacheEntry::PrototypeSlot{holder,holder_layout_revision,..})=>match &state.receiver_miss {None=>"receiver-proof-absent",Some((r,_)) if !r.ptr_eq(o)=>"receiver-identity-mismatch",Some((_,e)) if *e!=o.layout_revision()=>"receiver-layout-changed",_ if !o.prototype_is_weak(holder)=>"receiver-prototype-changed",_ if holder.upgrade().is_none_or(|h|h.layout_revision()!=*holder_layout_revision)=>"holder-layout-or-lifetime-changed",_=>"holder-slot-not-data"},
 Some(NamedPropertyCacheEntry::GrandprototypeSlot{parent,parent_layout_revision,holder,holder_layout_revision,..})=>match &state.receiver_miss {None=>"receiver-proof-absent",Some((r,_)) if !r.ptr_eq(o)=>"receiver-identity-mismatch",Some((_,e)) if *e!=o.layout_revision()=>"receiver-layout-changed",_ if super::grandprototype(o,parent,*parent_layout_revision,holder).is_none()=>"grandprototype-chain-or-layout-changed",_ if holder.upgrade().is_none_or(|h|h.layout_revision()!=*holder_layout_revision)=>"holder-layout-or-lifetime-changed",_=>"holder-slot-not-data"}
 }};
 let mut guards=Vec::new();
 if value.is_none() {
  let proto_entry=match entry {Some(NamedPropertyCacheEntry::PrototypeSlot{holder,..})=>Some(o.prototype_is_weak(holder)),Some(NamedPropertyCacheEntry::GrandprototypeSlot{parent,parent_layout_revision,holder,..})=>Some(super::grandprototype(o,parent,*parent_layout_revision,holder).is_some()),_=>None};
  if let Some(matches)=proto_entry {
   guards.push(if matches{"hot-holder-matches"}else{"hot-holder-differs"});
   if state.receiver_miss.as_ref().is_some_and(|(r,e)|r.ptr_eq(o)&&*e==o.layout_revision()){guards.push("receiver-proof-valid");}else{guards.push("receiver-proof-invalid");}
   let any=state.entries.iter().flatten().any(|e|match e {NamedPropertyCacheEntry::PrototypeSlot{holder,holder_layout_revision,..}=>o.prototype_is_weak(holder)&&holder.upgrade().is_some_and(|h|h.layout_revision()==*holder_layout_revision),NamedPropertyCacheEntry::GrandprototypeSlot{parent,parent_layout_revision,holder,holder_layout_revision,..}=>super::grandprototype(o,parent,*parent_layout_revision,holder).is_some_and(|h|h.layout_revision()==*holder_layout_revision),_=>false});
   guards.push(if any{"valid-holder-already-cached"}else{"holder-not-cached"});
  }
 }
 STORE.with(|s|{let mut s=s.borrow_mut();let Some(a)=s.active.last_mut().filter(|a|a.id==id)else{s.unscoped+=1;return;};if a.first_hot {s.sites.get_mut(&id).unwrap().repeated_hot+=1;return;}a.first_hot=true;let needs_observe=!a.observed;a.observed=true;if value.is_some(){a.answer=format!("hot-{}",entry.map(kind).unwrap_or("none"));}let key=s.sites[&id].key.clone();if needs_observe {s.observe(id,o,&key);}let row=s.sites.get_mut(&id).unwrap();if value.is_some(){row.hot_hits+=1;}else{row.hot_misses+=1;inc(&mut row.reasons,reason);for g in guards {inc(&mut row.guards,g);}}});
}
pub fn probe(cache:&NamedPropertyCache,result:&CacheProbe){let id=Rc::as_ptr(&cache.0) as usize;let state=cache.0.borrow();let family=state.entries.get(usize::from(state.hot.get())).and_then(|e|e.as_ref()).map(kind).unwrap_or("none");STORE.with(|s|{let mut s=s.borrow_mut();if let Some(a)=s.active.last_mut().filter(|a|a.id==id){match result {CacheProbe::Own(_)=>a.answer=format!("probe-own-{family}"),CacheProbe::PrototypeCandidate{..}=>a.answer="vm-prototype-own-miss".into(),CacheProbe::Miss=>{}}}});}
pub fn cache_event(cache:&NamedPropertyCache,ev:&str){let id=Rc::as_ptr(&cache.0) as usize;STORE.with(|s|{let mut s=s.borrow_mut();inc(&mut s.sites.entry(id).or_default().cache_events,ev);if let Some(a)=s.active.last_mut().filter(|a|a.id==id){if ev=="update"{a.answer="own-lookup".into();}else if ev=="update_from_prototype" && a.answer=="general-get"{a.answer="general-prototype-chain".into();}}});}
pub struct Call {key:String,read:Option<usize>}
pub fn call_begin(b:&Bytecode,pc:usize,tier:&str,reg:(usize,usize),callee:&Value,resolved:bool)->Call {STORE.with(|s|{let mut s=s.borrow_mut();let body=s.body(b);let key=format!("B{body}:pc{pc}:{tier}:{}",if resolved{"method"}else{"plain"});let read=if resolved {s.registers.get(&(tier.into(),reg.0,reg.1)).copied()}else{None};let row=s.calls.entry(key.clone()).or_default();inc(row,"executions");inc(row,&typ(callee));if let Some(id)=read{s.sites.get_mut(&id).unwrap().method_calls+=1;}Call{key,read}})}
pub fn call_outcome(c:&Call,kind:&str){STORE.with(|s|{let mut s=s.borrow_mut();inc(s.calls.get_mut(&c.key).unwrap(),kind);if let Some(id)=c.read{inc(&mut s.sites.get_mut(&id).unwrap().call_outcomes,kind);}});}
impl Drop for Store {fn drop(&mut self){for (id,r) in &self.sites {if r.executions==0{continue;} eprintln!("SITE{{\"id\":{id},\"label\":{},\"key\":{},\"executions\":{},\"hot_hits\":{},\"hot_misses\":{},\"repeated_hot\":{},\"method_calls\":{},\"reasons\":{},\"guards\":{},\"answers\":{},\"types\":{},\"inputs\":{},\"tiers\":{},\"shapes\":{},\"shape_count\":{},\"unordered_shape_count\":{},\"shared_name_shape_count\":{},\"literal_shape_count\":{},\"receiver_count\":{},\"epochs\":{},\"depths\":{},\"cache_events\":{},\"call_outcomes\":{}}}",j(&r.label),j(&r.key),r.executions,r.hot_hits,r.hot_misses,r.repeated_hot,r.method_calls,map(&r.reasons),map(&r.guards),map(&r.answers),map(&r.types),map(&r.inputs),map(&r.tiers),counts(&r.shapes),r.shapes.len(),r.unordered.len(),r.shared.len(),r.literal.len(),r.receivers.len(),counts(&r.epochs),counts(&r.depths),map(&r.cache_events),map(&r.call_outcomes));}
 for (id,shape) in self.shapes.iter().enumerate(){eprintln!("SHAPE{{\"id\":{id},\"description\":{}}}",j(shape));}for (key,c) in &self.calls{eprintln!("CALL{{\"label\":{},\"counts\":{}}}",j(key),map(c));}eprintln!("UNSCOPED{{\"hot_probes\":{}}}",self.unscoped);
}}
```
