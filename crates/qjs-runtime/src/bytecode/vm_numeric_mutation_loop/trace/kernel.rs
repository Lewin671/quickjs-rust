//! Static value-numbered representation for Numeric Trace leaf bodies.
//!
//! Compilation is deliberately fail-closed. Supported kernels run through the
//! preflighted executor; unsupported bodies retain the existing dense-loop
//! fallback.

use std::collections::{BTreeMap, BTreeSet};

use qjs_ast::BinaryOp;

use super::super::dense::{
    LocalWrite, MAX_DENSE_LOCALS, MAX_DENSE_OPS, NumberInstruction, Register,
};

mod executor;
pub(super) use executor::{KernelArrays, KernelScratch};

const MAX_POTENTIAL_ALIAS_PAIRS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ValueNumber(u8);

impl TryFrom<usize> for ValueNumber {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self(u8::try_from(value).map_err(|_| ())?))
    }
}

impl ValueNumber {
    #[inline(always)]
    fn index(self) -> usize {
        usize::from(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CellId(u8);

impl TryFrom<usize> for CellId {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self(u8::try_from(value).map_err(|_| ())?))
    }
}

impl CellId {
    #[inline(always)]
    fn index(self) -> usize {
        usize::from(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScratchSlot(u8);

impl TryFrom<usize> for ScratchSlot {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self(u8::try_from(value).map_err(|_| ())?))
    }
}

impl ScratchSlot {
    #[inline(always)]
    fn index(self) -> usize {
        usize::from(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AddressId {
    receiver: usize,
    index: ValueNumber,
}

// AddressId proves only static expression identity. The enclosing trace plan
// separately admits exactly `I` and `I + H`, whose entry algebra proves them
// distinct for every owned iteration.

#[derive(Clone, Debug, PartialEq, Eq)]
enum CanonicalOperation {
    Constant(u64),
    LoadLocal(usize),
    Binary {
        operation: BinaryOp,
        left: ValueNumber,
        right: ValueNumber,
    },
    InitialLoad(CellId),
}

#[derive(Clone, Copy, Debug)]
struct KernelLocalWrite {
    local: usize,
    value: ValueNumber,
}

#[derive(Clone, Copy, Debug)]
/// One statically identified cell; it does not prove run-time address uniqueness.
struct CellDescriptor {
    address: AddressId,
    index_slot: ScratchSlot,
    resolved_slot: ScratchSlot,
    initial_load: Option<ValueNumber>,
    #[cfg(test)]
    final_version: usize,
}

#[derive(Clone, Copy, Debug)]
struct StoreCommit {
    cell: CellId,
    value: ValueNumber,
}

#[derive(Clone, Copy, Debug)]
struct PendingStoreCommit {
    cell: CellId,
    value: ValueNumber,
    store_ordinal: usize,
    #[cfg(test)]
    source_operation: usize,
    #[cfg(test)]
    memory_version: usize,
}

#[derive(Clone, Copy, Debug)]
struct CellState {
    address: AddressId,
    current_value: Option<ValueNumber>,
    initial_load: Option<ValueNumber>,
    #[cfg(test)]
    version: usize,
    final_store: Option<PendingStoreCommit>,
}

#[derive(Clone, Debug)]
/// A kernel guarded by the enclosing trace's closed-form address proof.
pub(super) struct NumericDagKernel {
    operations: Vec<CanonicalOperation>,
    cells: Vec<CellDescriptor>,
    commits: Vec<StoreCommit>,
    local_writes: Vec<KernelLocalWrite>,
    counter_write: ValueNumber,
    unique_index_values: Vec<ValueNumber>,
    pure_mask: Vec<bool>,
    pure_values: Vec<ValueNumber>,
    dependent_values: Vec<ValueNumber>,
    #[cfg(test)]
    metadata: NumericDagKernelMetadata,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct NumericDagCellMetadata {
    pub(in super::super) receiver: usize,
    pub(in super::super) index_vn: usize,
    pub(in super::super) index_slot: usize,
    pub(in super::super) resolved_slot: usize,
    pub(in super::super) initial_load_vn: Option<usize>,
    pub(in super::super) final_memory_version: usize,
    pub(in super::super) final_store_ordinal: Option<usize>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct NumericDagCommitMetadata {
    pub(in super::super) cell: usize,
    pub(in super::super) value_vn: usize,
    pub(in super::super) store_ordinal: usize,
    pub(in super::super) source_operation: usize,
    pub(in super::super) memory_version: usize,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct NumericDagKernelMetadata {
    pub(in super::super) raw_source_operations: usize,
    pub(in super::super) canonical_operations: usize,
    pub(in super::super) executable_operations: usize,
    pub(in super::super) pure_prefix_operations: usize,
    pub(in super::super) logical_loads: usize,
    pub(in super::super) logical_stores: usize,
    pub(in super::super) unique_index_vns: usize,
    pub(in super::super) unique_cells: usize,
    pub(in super::super) potential_alias_pairs: usize,
    pub(in super::super) initial_load_cells: usize,
    pub(in super::super) commit_events: usize,
    pub(in super::super) local_write_count: usize,
    pub(in super::super) local_write_slots: Vec<usize>,
    pub(in super::super) cell_descriptors: Vec<NumericDagCellMetadata>,
    pub(in super::super) commit_descriptors: Vec<NumericDagCommitMetadata>,
    pub(in super::super) store_ordinals: Vec<usize>,
}

impl NumericDagKernel {
    pub(super) fn preflight_dependencies_are_pure(&self, schedule_reads: &BTreeSet<usize>) -> bool {
        if !self.pure_mask[self.counter_write.index()] {
            return false;
        }
        let mut sensitive_locals = schedule_reads.clone();
        sensitive_locals.extend(self.operations.iter().enumerate().filter_map(
            |(value, operation)| match operation {
                CanonicalOperation::LoadLocal(local) if self.pure_mask[value] => Some(*local),
                CanonicalOperation::Constant(_)
                | CanonicalOperation::LoadLocal(_)
                | CanonicalOperation::Binary { .. }
                | CanonicalOperation::InitialLoad(_) => None,
            },
        ));
        self.local_writes.iter().all(|write| {
            !sensitive_locals.contains(&write.local) || self.pure_mask[write.value.index()]
        })
    }

    pub(super) fn matches_counted_index_shape(
        &self,
        inner_counter: usize,
        outer_counter: usize,
    ) -> bool {
        let Some(inner) = self.find_operation(|operation| {
            matches!(operation, CanonicalOperation::LoadLocal(local) if *local == inner_counter)
        }) else {
            return false;
        };
        let Some(outer) = self.find_operation(|operation| {
            matches!(operation, CanonicalOperation::LoadLocal(local) if *local == outer_counter)
        }) else {
            return false;
        };
        let Some(one) = self.find_operation(|operation| {
            matches!(operation, CanonicalOperation::Constant(bits) if *bits == 1.0_f64.to_bits())
        }) else {
            return false;
        };
        let Some(outer_stride) = self.find_operation(|operation| {
            matches!(
                operation,
                CanonicalOperation::Binary {
                    operation: BinaryOp::Shl,
                    left,
                    right,
                } if *left == outer && *right == one
            )
        }) else {
            return false;
        };
        let Some(offset) = self.find_operation(|operation| {
            matches!(
                operation,
                CanonicalOperation::Binary {
                    operation: BinaryOp::Add,
                    left,
                    right,
                } if *left == inner && *right == outer
            )
        }) else {
            return false;
        };
        let Some(next_inner) = self.find_operation(|operation| {
            matches!(
                operation,
                CanonicalOperation::Binary {
                    operation: BinaryOp::Add,
                    left,
                    right,
                } if *left == inner && *right == outer_stride
            )
        }) else {
            return false;
        };
        let mut inner_writes = self
            .local_writes
            .iter()
            .filter(|write| write.local == inner_counter);
        self.counter_write == next_inner
            && self.unique_index_values.len() == 2
            && self.unique_index_values.contains(&inner)
            && self.unique_index_values.contains(&offset)
            && inner_writes
                .next()
                .is_some_and(|write| write.value == next_inner)
            && inner_writes.next().is_none()
    }

    fn find_operation(
        &self,
        predicate: impl Fn(&CanonicalOperation) -> bool,
    ) -> Option<ValueNumber> {
        self.operations
            .iter()
            .position(predicate)
            .and_then(|index| ValueNumber::try_from(index).ok())
    }
}

#[cfg(test)]
impl NumericDagKernel {
    pub(super) fn metadata(&self) -> &NumericDagKernelMetadata {
        &self.metadata
    }
}

#[derive(Default)]
struct KernelCompiler {
    operations: Vec<CanonicalOperation>,
    source_values: Vec<ValueNumber>,
    cell_ids: BTreeMap<AddressId, CellId>,
    cells: Vec<CellState>,
    #[cfg(test)]
    logical_loads: usize,
    logical_stores: usize,
    unique_index_values: BTreeSet<ValueNumber>,
}

impl KernelCompiler {
    fn intern(&mut self, operation: CanonicalOperation) -> Option<ValueNumber> {
        if let Some(index) = self
            .operations
            .iter()
            .position(|candidate| candidate == &operation)
        {
            return ValueNumber::try_from(index).ok();
        }
        let value = ValueNumber::try_from(self.operations.len()).ok()?;
        self.operations.push(operation);
        Some(value)
    }

    fn source_value(&self, register: Register) -> Option<ValueNumber> {
        self.source_values.get(register).copied()
    }

    fn cell_for(&mut self, address: AddressId) -> Option<CellId> {
        if let Some(cell) = self.cell_ids.get(&address) {
            return Some(*cell);
        }
        let cell = CellId::try_from(self.cells.len()).ok()?;
        self.cell_ids.insert(address, cell);
        self.cells.push(CellState {
            address,
            current_value: None,
            initial_load: None,
            #[cfg(test)]
            version: 0,
            final_store: None,
        });
        Some(cell)
    }

    fn initial_load(&mut self, cell: CellId) -> Option<ValueNumber> {
        if let Some(value) = self.cells[cell.index()].initial_load {
            return Some(value);
        }
        #[cfg(test)]
        debug_assert_eq!(self.cells[cell.index()].version, 0);
        let value = self.intern(CanonicalOperation::InitialLoad(cell))?;
        let state = &mut self.cells[cell.index()];
        state.initial_load = Some(value);
        state.current_value = Some(value);
        Some(value)
    }

    fn compile_operation(
        &mut self,
        _source_operation: usize,
        operation: &NumberInstruction,
    ) -> Option<ValueNumber> {
        match operation {
            NumberInstruction::Constant(value) => {
                self.intern(CanonicalOperation::Constant(value.to_bits()))
            }
            NumberInstruction::LoadLocal(local) => {
                if *local >= MAX_DENSE_LOCALS {
                    return None;
                }
                self.intern(CanonicalOperation::LoadLocal(*local))
            }
            NumberInstruction::Binary {
                operation,
                left,
                right,
            } if supported_binary(*operation) => {
                let left = self.source_value(*left)?;
                let right = self.source_value(*right)?;
                self.intern(CanonicalOperation::Binary {
                    operation: *operation,
                    left,
                    right,
                })
            }
            NumberInstruction::DenseLoad { receiver, index } => {
                let index = self.source_value(*index)?;
                self.unique_index_values.insert(index);
                #[cfg(test)]
                {
                    self.logical_loads += 1;
                }
                let cell = self.cell_for(AddressId {
                    receiver: *receiver,
                    index,
                })?;
                self.cells[cell.index()]
                    .current_value
                    .or_else(|| self.initial_load(cell))
            }
            NumberInstruction::DenseStore {
                receiver,
                index,
                value,
            } => {
                let index = self.source_value(*index)?;
                let value = self.source_value(*value)?;
                self.unique_index_values.insert(index);
                let cell = self.cell_for(AddressId {
                    receiver: *receiver,
                    index,
                })?;
                let store_ordinal = self.logical_stores;
                self.logical_stores += 1;
                #[cfg(test)]
                let memory_version = self.cells[cell.index()].version.checked_add(1)?;
                let commit = PendingStoreCommit {
                    cell,
                    value,
                    store_ordinal,
                    #[cfg(test)]
                    source_operation: _source_operation,
                    #[cfg(test)]
                    memory_version,
                };
                let state = &mut self.cells[cell.index()];
                state.current_value = Some(value);
                #[cfg(test)]
                {
                    state.version = memory_version;
                }
                state.final_store = Some(commit);
                Some(value)
            }
            NumberInstruction::LoadInvariant(_)
            | NumberInstruction::Binary { .. }
            | NumberInstruction::Unary { .. }
            | NumberInstruction::Update { .. }
            | NumberInstruction::MathRound { .. } => None,
        }
    }
}

pub(super) fn compile_numeric_dag_kernel(
    operations: &[NumberInstruction],
    writes: &[LocalWrite],
    counter_write: Register,
    store_count: usize,
) -> Option<NumericDagKernel> {
    if operations.len() > MAX_DENSE_OPS {
        return None;
    }
    let mut compiler = KernelCompiler::default();
    for (source_operation, operation) in operations.iter().enumerate() {
        let value = compiler.compile_operation(source_operation, operation)?;
        compiler.source_values.push(value);
    }
    if compiler.logical_stores != store_count {
        return None;
    }
    let local_writes = writes
        .iter()
        .map(|write| {
            if write.local >= MAX_DENSE_LOCALS {
                return None;
            }
            Some(KernelLocalWrite {
                local: write.local,
                value: compiler.source_value(write.value)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let counter_write = compiler.source_value(counter_write)?;
    #[cfg(test)]
    let leading_pure_prefix_len = compiler
        .operations
        .iter()
        .position(|operation| matches!(operation, CanonicalOperation::InitialLoad(_)))
        .unwrap_or(compiler.operations.len());
    let mut pure_mask = vec![false; compiler.operations.len()];
    for (value, operation) in compiler.operations.iter().enumerate() {
        pure_mask[value] = match operation {
            CanonicalOperation::Constant(_) | CanonicalOperation::LoadLocal(_) => true,
            CanonicalOperation::Binary { left, right, .. } => {
                pure_mask[left.index()] && pure_mask[right.index()]
            }
            CanonicalOperation::InitialLoad(_) => false,
        };
    }
    if !compiler
        .unique_index_values
        .iter()
        .all(|index| pure_mask[index.index()])
    {
        return None;
    }
    let pure_values = pure_mask
        .iter()
        .enumerate()
        .filter_map(|(value, pure)| pure.then(|| ValueNumber::try_from(value).ok()).flatten())
        .collect::<Vec<_>>();
    let dependent_values = compiler
        .operations
        .iter()
        .enumerate()
        .filter_map(|(value, operation)| {
            (!pure_mask[value] && !matches!(operation, CanonicalOperation::InitialLoad(_)))
                .then(|| ValueNumber::try_from(value).ok())
                .flatten()
        })
        .collect::<Vec<_>>();
    let unique_index_values: Vec<_> = compiler.unique_index_values.iter().copied().collect();
    if unique_index_values
        .len()
        .checked_add(compiler.cells.len())?
        > MAX_DENSE_OPS
    {
        return None;
    }
    let cells = compiler
        .cells
        .iter()
        .enumerate()
        .map(|(cell_id, state)| {
            Some(CellDescriptor {
                address: state.address,
                index_slot: ScratchSlot::try_from(
                    unique_index_values
                        .binary_search(&state.address.index)
                        .ok()?,
                )
                .ok()?,
                resolved_slot: ScratchSlot::try_from(unique_index_values.len() + cell_id).ok()?,
                initial_load: state.initial_load,
                #[cfg(test)]
                final_version: state.version,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let mut potential_alias_pairs = 0_usize;
    for right in 0..cells.len() {
        for left in 0..right {
            if cells[left].address.receiver == cells[right].address.receiver {
                if potential_alias_pairs == MAX_POTENTIAL_ALIAS_PAIRS {
                    return None;
                }
                potential_alias_pairs += 1;
            }
        }
    }
    let mut pending_commits: Vec<_> = compiler
        .cells
        .iter()
        .filter_map(|state| state.final_store)
        .collect();
    // Earlier stores to the exact same (receiver, index-VN) cell have already
    // fed every dependent load through memory SSA, so only that cell's final
    // value needs publishing. This relies on the caller's distinct ordinary
    // dense-array lease and on the admitted body having no observable hooks.
    pending_commits.sort_unstable_by_key(|commit| commit.store_ordinal);
    debug_assert!(
        pending_commits
            .windows(2)
            .all(|pair| pair[0].store_ordinal < pair[1].store_ordinal)
    );
    #[cfg(test)]
    let metadata = {
        let initial_load_cells = cells
            .iter()
            .filter(|cell| cell.initial_load.is_some())
            .count();
        // InitialLoad is an executable realization of a logical DenseLoad,
        // not an extra canonical source operation. Conversely, every logical
        // load/store remains visible in this source-level count even when
        // memory SSA forwards a load or discards an overwritten store.
        let canonical_operations = compiler.operations.len() - initial_load_cells
            + compiler.logical_loads
            + compiler.logical_stores;
        NumericDagKernelMetadata {
            raw_source_operations: operations.len(),
            canonical_operations,
            executable_operations: compiler.operations.len(),
            pure_prefix_operations: leading_pure_prefix_len,
            logical_loads: compiler.logical_loads,
            logical_stores: compiler.logical_stores,
            unique_index_vns: compiler.unique_index_values.len(),
            unique_cells: cells.len(),
            potential_alias_pairs,
            initial_load_cells,
            commit_events: pending_commits.len(),
            local_write_count: local_writes.len(),
            local_write_slots: local_writes.iter().map(|write| write.local).collect(),
            cell_descriptors: cells
                .iter()
                .enumerate()
                .map(|(cell_id, cell)| NumericDagCellMetadata {
                    receiver: cell.address.receiver,
                    index_vn: cell.address.index.index(),
                    index_slot: cell.index_slot.index(),
                    resolved_slot: cell.resolved_slot.index(),
                    initial_load_vn: cell.initial_load.map(ValueNumber::index),
                    final_memory_version: cell.final_version,
                    final_store_ordinal: pending_commits
                        .iter()
                        .find(|commit| {
                            commit.cell
                                == CellId::try_from(cell_id)
                                    .expect("compiled cell identifiers fit in u8")
                        })
                        .map(|commit| commit.store_ordinal),
                })
                .collect(),
            commit_descriptors: pending_commits
                .iter()
                .map(|commit| NumericDagCommitMetadata {
                    cell: commit.cell.index(),
                    value_vn: commit.value.index(),
                    store_ordinal: commit.store_ordinal,
                    source_operation: commit.source_operation,
                    memory_version: commit.memory_version,
                })
                .collect(),
            store_ordinals: pending_commits
                .iter()
                .map(|commit| commit.store_ordinal)
                .collect(),
        }
    };
    let commits = pending_commits
        .into_iter()
        .map(|commit| StoreCommit {
            cell: commit.cell,
            value: commit.value,
        })
        .collect();
    Some(NumericDagKernel {
        operations: compiler.operations,
        cells,
        commits,
        local_writes,
        counter_write,
        unique_index_values,
        pure_mask,
        pure_values,
        dependent_values,
        #[cfg(test)]
        metadata,
    })
}

fn supported_binary(operation: BinaryOp) -> bool {
    matches!(
        operation,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseXor
            | BinaryOp::BitwiseOr
    )
}
