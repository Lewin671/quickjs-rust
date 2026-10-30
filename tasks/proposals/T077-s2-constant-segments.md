# S2 observer-free field publication and monomorphic leaf ABI

Frozen before implementation. At `90719950`, job
`20261030T190528Z-t077-s2-r2-construction-roots-maps`, class-field raytrace
still takes 1.09458 main instructions and cdjs 1.05140 despite only 783/759
transition misses. String slice remains 1.08340 after separating the complete
resolution miss frame; the data helper still includes all polymorphic arms.

For primitive constant class-field segments already admitted as observer-free,
retain an old-layout/final-layout append plan on the first field's existing
metadata allocation. Guard an ordinary extensible receiver and the old shared
layout. Reserve once, fill values in definition order, then publish the final
layout after the segment completes. Admission proves every definition succeeds
and invokes no JS; no future field is installed before an effectful initializer.
Duplicates, redefinitions, symbols, exotic receivers and private layouts keep
sequential definitions. Effectful initializers continue publishing each field
only after evaluation. A hot segment acquires only its one final owning layout.

Keep single-field and segment plans together in the existing Rc metadata cell;
retain only string schema metadata, never receiver/JS values. Heap slot extension
may borrow its Vec once rather than repeatedly moving the storage enum. Inline
pairs retain their allocation-free representation. The sole named authority and
class field observer order remain unchanged.

Split monomorphic data loads from polymorphic/general cache handling, retaining
an inline layout/version guard and live slot load. Put the larger alternatives
behind a separate helper; do not restore exact-value or receiver caches. Check
polymorphic/inherited sentinels as well as primitive reads for code-size tradeoffs.

Focused observer/descriptor/cache tests, strict checks and a new paired counter
cohort precede payoff claims. Main remains `dcee3c56`; no unsafe, dependency,
benchmark template, binding change or held-out input. Array migration is queued.
