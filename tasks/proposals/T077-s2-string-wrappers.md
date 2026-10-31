# S2 boxed String construction and ordinary length reads

Freeze before implementation at 01ee5ee6. Public86 external remains 1.035225
wall/1.017096 instructions. Non-timing job
`20261030T231113Z-t077-s2-r2-public-regression-counts` has date-format-xparb:
base/candidate 340,980,360/430,619,728 median instructions; 20,678/20,143 ordinary
objects, 158,625/220,592 allocator requests, candidate 40,459 transition hits.
The public source creates five String wrappers per formatting iteration.

Canonical sample job `20261030T231114Z-t077-s2-r2-date-slot-profiles` scales
only that input's original loop bound 4,000 to 400,000 for sampling, never payoff
instructions/timing. Candidate 4,062 main-thread samples: allocation 142 self,
AtomDomain.find 178, NamedStore.get_text 104, descriptor 73. Metadata fallback
is only 10 self samples. Preserve original/scaled hashes and exact image hashes.

Source confirms three avoidable costs. NamedTarget incorrectly groups boxed
String `length` with array length/index hooks, although String length is an
ordinary immutable own data property. StringData uses the large general cold
payload. The primitive-argument String constructor creates flavor 0 with a
length, then replays that field into flavor 3 and resolves the prototype twice.

1. Admit boxed String ordinary `length` through the existing shape/data-slot IC;
   only String indices retain exotic routing. Read-only writes still decline.
2. Add a small typed String cold payload with optional stable general extensions,
   like Date; remove StringData from ObjectColdData. Production branding is on
   fresh receivers (bootstrap String.prototype, native/String fast construction,
   abstract boxing). Preserve private String updates internally and all public
   index, brand, length, reflection, subclass and coercion behavior.
3. Build primitive String wrappers with their final flavor 3, length descriptor
   and private data before exposure. A realm-owned intrinsic template keeps only
   the fixed length layout and its already-owned intrinsic prototype edge; each
   object supplies its own length/text. No per-object root/hash/transition or
   extra layout clone on a hit. Unknown prototype/domain uses common NamedStore
   construction with the same final flavor. Do not pin arbitrary old prototypes.

Reuse NamedStore/inline ValueSlots. A small family-specific layout template is
justified by removing repeated native-wrapper construction/rebase work; it owns
no receiver, callee or cached JS values. No dependency, unsafe, native-entry/S3
or binding API change, source matcher, held-out/formal run. Test UTF-16 indices,
readonly length ICs, forged keys, prototype changes, private subclasses and cold
extension identity; run String/IC/full gates and fresh public work/triage.
