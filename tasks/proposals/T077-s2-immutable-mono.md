# S2 immutable monomorphic IC authority

Freeze before implementation at f380089d. Completed 14abe3f3 public triage
`20261030T222542Z-t077-s2-r2-header-public86` still loses: development .983161,
external 1.035770 and public selection 1.010875 wall. Instructions remain
1.201732 nbody, 1.258266 date-format-xparb, 1.036877 papaparse and 1.065389
string_slice. The construction cohorts measure increased storage borrows;
source audit additionally finds one mutable IC-state RefCell protocol on every
monomorphic hit, absent from main's inline per-site slot metadata. This extra
IC borrow is not included in the diagnostic storage-borrow counter.

Lift that structural cost: initialize the first monomorphic Entry once in an
immutable OnceCell owned by the existing Rc cache handle. State's Mono marker
refers to that one authority. A Cell marks whether it is active; hot reads and
numeric overwrites inspect its immutable guards and borrow only live storage.
Before any metadata replacement/promotion disable the hot marker. Later mono
replacement/polymorphic/mega metadata uses the existing RefCell state and the
same semantic guard/load/write APIs. No cached Value or strong receiver edge.

Promotion copies only weak/versioned metadata into the four active ways. The
inactive first token remains bounded at one per site until cache destruction;
it never admits a fifth active way, pins a receiver or preserves payloads.
Clear/reuse must not reactivate the old token. Document this fixed weak-token
cost and measure cache/header allocations alongside work/RSS. Native/S3 API,
binding cells, object store and invalidation generations are unchanged.

Prove cold/hot transitions, value type changes, shared cache clones, prototype
invalidation, old-token clear/reuse and mono/four-way/mega behavior in debug IC
verification. Measure general public cases and final cohorts; no unsafe,
dependency, held-out or address/layout selection. Retain only with the stage's
required correctness and payoff evidence.
