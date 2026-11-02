# Cleared frame storage

RuntimeServices' StackPool owns reusable register/header stacks and one
GeneralPool handle for operand/local vectors and cold boxes. All four families
share one count/byte authority: at most 64 free records, 256 KiB including
conservative directory/header charges, and 16 KiB per record. General vectors
retain at most 256 elements and 32 records of either kind; cold retains at most
eight boxes. Cold charges include every retained inner vector after reset.
Bytecode owns no allocation recycler or mutable free-capacity state.

An active general frame retains the data-only handle in OperandStack. Its
owner's Drop clears and returns private locals/cold state; OperandStack returns
its values separately. Return, throw, abandonment and suspension teardown use
the same ownership rule. Runtime teardown drains every free bin even if a
suspended record retains a handle. The handle owns no RuntimeServices, realm,
Context or code, and cannot keep an execution facade alive.

Values and semantic owners are released before borrowing a bin. Resource
teardown may drop another frame using the same pool. Only cleared capacity
enters a bin; active/native-borrowed/suspended storage is neither counted as
free nor evicted. Cells, scope leases, mapped arguments and closures retain
independent identities and never point into a reusable vector.

Local capacity may cross bodies and lengths. Take supplies exactly the current
length of None slots before BindingPlan initialization. Debug poison is checked
and cleared before resizing; it never reaches an escaped cell. Cold reset drops
references, handlers, class/disposal owners and pending completions while
retaining only bounded empty vector capacity. Ordinary inline calls still
push/pop their owned contiguous stack without a pool or directory lookup.

Stack-IR and register banks retain their respective semantic encodings; S3e
owns universal Code lowering. See [execution](core-rebuild-execution.md) and
[suspension](generator-suspension.md) for the owners that use this storage.
