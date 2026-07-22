//! Control-flow and alias analysis foundation for virtual object lowering.
//!
//! This pass is intentionally analysis-only. It proves which literal
//! allocation sites have no observable identity before a later change teaches
//! the VM how to scalar-replace them. Keeping the proof separate from the
//! rewrite makes every unsupported bytecode effect fail closed and prevents a
//! benchmark-shaped instruction matcher from becoming a semantic shortcut.

#![allow(dead_code)]

use std::{
    collections::{BTreeSet, VecDeque},
    rc::Rc,
};

use crate::{Value, value::ObjectLiteralShape};

use super::ir::{ArrayElementKind, Bytecode, Op};

type CandidateId = usize;

#[derive(Clone, Debug)]
pub(super) struct VirtualObjectAnalysis {
    pub(super) cfg: ControlFlowGraph,
    pub(super) slot_authority: SlotAuthority,
    pub(super) candidates: Vec<VirtualCandidate>,
    pub(super) complete: bool,
}

#[derive(Clone, Debug)]
pub(super) struct VirtualCandidate {
    pub(super) allocation_ip: usize,
    pub(super) kind: VirtualKind,
    pub(super) uses: BTreeSet<VirtualUse>,
    pub(super) escape_reasons: BTreeSet<EscapeReason>,
    seen: bool,
}

impl VirtualCandidate {
    pub(super) fn is_virtualizable(&self) -> bool {
        self.seen && self.escape_reasons.is_empty()
    }
}

#[derive(Clone, Debug)]
pub(super) enum VirtualKind {
    Object(Rc<ObjectLiteralShape>),
    DenseArray { length: usize },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum VirtualUse {
    Alias { ip: usize, slot: usize },
    Discard { ip: usize },
    FieldRead { ip: usize, input: usize },
    FieldWrite { ip: usize, input: usize },
    ObjectGuard { ip: usize },
    ArrayLengthRead { ip: usize },
    ElementRead { ip: usize, index: usize },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum EscapeReason {
    AmbiguousAlias { ip: usize },
    DynamicScope,
    HomeObjectSideEffect { ip: usize },
    IdentityUse { ip: usize },
    InvalidControlFlow,
    InvalidStack { ip: usize },
    StoredInAggregate { ip: usize },
    Suspension { ip: usize },
    UnknownProperty { ip: usize },
    UnsafeSlot { ip: usize, slot: usize },
    UnsupportedInstruction { ip: usize },
    Unreachable,
}

#[derive(Clone, Debug)]
pub(super) struct SlotAuthority {
    authoritative: Vec<bool>,
    pub(super) dynamic_scope: bool,
}

impl SlotAuthority {
    fn for_bytecode(bytecode: &Bytecode) -> Self {
        let dynamic_scope = bytecode.code.iter().any(|op| {
            matches!(
                op,
                Op::CallDirectEval { .. }
                    | Op::CallDirectEvalSpread { .. }
                    | Op::EnterWith
                    | Op::ExitWith
                    | Op::LoadIdentWith { .. }
                    | Op::ResolveIdentWith { .. }
                    | Op::LoadResolvedIdentWith { .. }
                    | Op::StoreIdentWith { .. }
                    | Op::StoreResolvedIdentWith { .. }
                    | Op::TypeofIdentWith { .. }
                    | Op::DeleteIdentWith { .. }
            )
        });
        let mut captured = vec![false; bytecode.locals.len()];
        let mut class_may_capture = false;
        for op in &bytecode.code {
            match op {
                Op::NewFunction {
                    lexical_captures, ..
                } => {
                    for (_, slot) in lexical_captures {
                        if let Some(captured) = captured.get_mut(*slot) {
                            *captured = true;
                        }
                    }
                }
                Op::FreshIterationScope(slots) => {
                    for slot in slots {
                        if let Some(captured) = captured.get_mut(*slot) {
                            *captured = true;
                        }
                    }
                }
                // Class element capture metadata is distributed across the
                // constructor and element thunks. Until it is normalized into
                // one table, reject local aliases in a body containing a
                // class rather than missing a less common capture route.
                Op::NewClass { .. } => class_may_capture = true,
                _ => {}
            }
        }
        let authoritative = (0..bytecode.locals.len())
            .map(|slot| {
                !bytecode.global_scope
                    && !dynamic_scope
                    && !class_may_capture
                    && !captured[slot]
                    && !bytecode.local_is_parameter(slot)
                    && !bytecode.locals[slot].is_received_upvalue()
                    && !bytecode.local_is_sloppy_global_fallback(slot)
                    && !bytecode.local_is_eval_deletable(slot)
            })
            .collect();
        Self {
            authoritative,
            dynamic_scope,
        }
    }

    pub(super) fn is_authoritative(&self, slot: usize) -> bool {
        self.authoritative.get(slot).copied().unwrap_or(false)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ControlFlowGraph {
    pub(super) blocks: Vec<BasicBlock>,
    instruction_blocks: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BasicBlock {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) successors: Vec<usize>,
    pub(super) predecessors: Vec<usize>,
}

impl ControlFlowGraph {
    fn build(code: &[Op]) -> Result<Self, ()> {
        if code.is_empty() {
            return Ok(Self {
                blocks: Vec::new(),
                instruction_blocks: Vec::new(),
            });
        }
        let mut leaders = BTreeSet::from([0]);
        for (ip, op) in code.iter().enumerate() {
            let target = match op {
                Op::Jump(target)
                | Op::JumpIfFalse(target)
                | Op::JumpIfTrue(target)
                | Op::JumpIfNotNullish(target)
                | Op::AbruptJump(target) => Some(*target),
                _ => None,
            };
            if let Some(target) = target {
                if target >= code.len() {
                    return Err(());
                }
                leaders.insert(target);
            }
            if (target.is_some() || is_terminal(op)) && ip + 1 < code.len() {
                leaders.insert(ip + 1);
            }
        }

        let leaders = leaders.into_iter().collect::<Vec<_>>();
        let mut blocks = leaders
            .iter()
            .enumerate()
            .map(|(index, start)| BasicBlock {
                start: *start,
                end: leaders.get(index + 1).copied().unwrap_or(code.len()),
                successors: Vec::new(),
                predecessors: Vec::new(),
            })
            .collect::<Vec<_>>();
        let mut instruction_blocks = vec![0; code.len()];
        for (block, range) in blocks.iter().enumerate() {
            instruction_blocks[range.start..range.end].fill(block);
        }

        for block in 0..blocks.len() {
            let last_ip = blocks[block].end - 1;
            let fallthrough = (block + 1 < blocks.len()).then_some(block + 1);
            let mut successors = match &code[last_ip] {
                Op::Jump(target) | Op::AbruptJump(target) => {
                    vec![instruction_blocks[*target]]
                }
                Op::JumpIfFalse(target) | Op::JumpIfTrue(target) | Op::JumpIfNotNullish(target) => {
                    let mut successors = vec![instruction_blocks[*target]];
                    if let Some(fallthrough) = fallthrough {
                        successors.push(fallthrough);
                    }
                    successors
                }
                op if is_terminal(op) => Vec::new(),
                _ => fallthrough.into_iter().collect(),
            };
            successors.sort_unstable();
            successors.dedup();
            blocks[block].successors = successors;
        }
        for block in 0..blocks.len() {
            let successors = blocks[block].successors.clone();
            for successor in successors {
                blocks[successor].predecessors.push(block);
            }
        }
        Ok(Self {
            blocks,
            instruction_blocks,
        })
    }
}

fn is_terminal(op: &Op) -> bool {
    matches!(op, Op::Return | Op::Throw | Op::ThrowReferenceError(_))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AbstractValue {
    candidates: BTreeSet<CandidateId>,
    may_be_other: bool,
    may_be_home_function: bool,
    known_string: Option<Rc<String>>,
}

impl AbstractValue {
    fn unknown() -> Self {
        Self {
            candidates: BTreeSet::new(),
            may_be_other: true,
            may_be_home_function: true,
            known_string: None,
        }
    }

    fn known_non_function() -> Self {
        Self {
            candidates: BTreeSet::new(),
            may_be_other: true,
            may_be_home_function: false,
            known_string: None,
        }
    }

    fn known_string(value: Rc<String>) -> Self {
        Self {
            candidates: BTreeSet::new(),
            may_be_other: true,
            may_be_home_function: false,
            known_string: Some(value),
        }
    }

    fn known_function() -> Self {
        Self::unknown()
    }

    fn virtual_candidate(candidate: CandidateId) -> Self {
        Self {
            candidates: BTreeSet::from([candidate]),
            may_be_other: false,
            may_be_home_function: false,
            known_string: None,
        }
    }

    fn exact_candidate(&self) -> Option<CandidateId> {
        (!self.may_be_other && self.candidates.len() == 1)
            .then(|| *self.candidates.first().expect("one candidate checked"))
    }

    fn join(&self, other: &Self) -> Self {
        let mut candidates = self.candidates.clone();
        candidates.extend(other.candidates.iter().copied());
        Self {
            candidates,
            may_be_other: self.may_be_other || other.may_be_other,
            may_be_home_function: self.may_be_home_function || other.may_be_home_function,
            known_string: (self.known_string == other.known_string)
                .then(|| self.known_string.clone())
                .flatten(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FlowState {
    stack: Vec<AbstractValue>,
    locals: Vec<AbstractValue>,
}

impl FlowState {
    fn entry(local_count: usize) -> Self {
        Self {
            stack: Vec::new(),
            locals: vec![AbstractValue::unknown(); local_count],
        }
    }

    fn join(&self, other: &Self, ip: usize) -> Result<Self, AnalysisFailure> {
        if self.stack.len() != other.stack.len() || self.locals.len() != other.locals.len() {
            return Err(AnalysisFailure::InvalidStack(ip));
        }
        Ok(Self {
            stack: self
                .stack
                .iter()
                .zip(&other.stack)
                .map(|(left, right)| left.join(right))
                .collect(),
            locals: self
                .locals
                .iter()
                .zip(&other.locals)
                .map(|(left, right)| left.join(right))
                .collect(),
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum AnalysisFailure {
    InvalidStack(usize),
    Unsupported(usize),
}

struct Analyzer<'a> {
    bytecode: &'a Bytecode,
    authority: SlotAuthority,
    candidates: Vec<VirtualCandidate>,
    candidate_at: Vec<Option<CandidateId>>,
}

impl<'a> Analyzer<'a> {
    fn new(bytecode: &'a Bytecode) -> Self {
        let mut candidates = Vec::new();
        let mut candidate_at = vec![None; bytecode.code.len()];
        for (ip, op) in bytecode.code.iter().enumerate() {
            let kind = match op {
                Op::NewObjectDataLiteral { shape } => Some(VirtualKind::Object(shape.clone())),
                Op::NewArray { elements }
                    if elements
                        .iter()
                        .all(|element| matches!(element, ArrayElementKind::Expr)) =>
                {
                    Some(VirtualKind::DenseArray {
                        length: elements.len(),
                    })
                }
                _ => None,
            };
            if let Some(kind) = kind {
                let candidate = candidates.len();
                candidate_at[ip] = Some(candidate);
                candidates.push(VirtualCandidate {
                    allocation_ip: ip,
                    kind,
                    uses: BTreeSet::new(),
                    escape_reasons: BTreeSet::new(),
                    seen: false,
                });
            }
        }
        Self {
            bytecode,
            authority: SlotAuthority::for_bytecode(bytecode),
            candidates,
            candidate_at,
        }
    }

    fn mark_all(&mut self, reason: EscapeReason) {
        for candidate in &mut self.candidates {
            candidate.escape_reasons.insert(reason.clone());
        }
    }

    fn escape_value(&mut self, value: &AbstractValue, reason: EscapeReason) {
        for candidate in &value.candidates {
            self.candidates[*candidate]
                .escape_reasons
                .insert(reason.clone());
        }
    }

    fn escape_state(&mut self, state: &FlowState, reason: EscapeReason) {
        for value in state.stack.iter().chain(&state.locals) {
            self.escape_value(value, reason.clone());
        }
    }

    fn pop(state: &mut FlowState, ip: usize) -> Result<AbstractValue, AnalysisFailure> {
        state.stack.pop().ok_or(AnalysisFailure::InvalidStack(ip))
    }

    fn pop_many(
        state: &mut FlowState,
        count: usize,
        ip: usize,
    ) -> Result<Vec<AbstractValue>, AnalysisFailure> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(Self::pop(state, ip)?);
        }
        Ok(values)
    }

    fn transfer(&mut self, ip: usize, state: &mut FlowState) -> Result<(), AnalysisFailure> {
        let op = self.bytecode.code[ip].clone();
        match op {
            Op::LoadConst(index) => {
                let value = self
                    .bytecode
                    .constants
                    .get(index)
                    .ok_or(AnalysisFailure::InvalidStack(ip))?;
                state.stack.push(match value {
                    Value::Function(_) => AbstractValue::known_function(),
                    Value::String(value) => AbstractValue::known_string(value.clone()),
                    _ => AbstractValue::known_non_function(),
                });
            }
            Op::LoadLocal(slot) | Op::LoadLocalOrUndefined(slot) => {
                let value = if self.authority.is_authoritative(slot) {
                    state.locals.get(slot).cloned()
                } else {
                    Some(AbstractValue::unknown())
                }
                .ok_or(AnalysisFailure::InvalidStack(ip))?;
                state.stack.push(value);
            }
            Op::LoadNewTarget | Op::LoadGlobal(_) => {
                state.stack.push(AbstractValue::unknown());
            }
            Op::TypeofGlobal(_) => state.stack.push(AbstractValue::known_non_function()),
            Op::AppendStringLiteralLocal { slot, .. } => {
                let previous = state
                    .locals
                    .get(slot)
                    .cloned()
                    .ok_or(AnalysisFailure::InvalidStack(ip))?;
                self.escape_value(&previous, EscapeReason::IdentityUse { ip });
                state.locals[slot] = AbstractValue::known_non_function();
            }
            Op::AppendStringLiteralGlobal { .. } => {}
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                let value = Self::pop(state, ip)?;
                if self.authority.is_authoritative(slot) {
                    let target = state
                        .locals
                        .get_mut(slot)
                        .ok_or(AnalysisFailure::InvalidStack(ip))?;
                    *target = value.clone();
                    for candidate in &value.candidates {
                        self.candidates[*candidate]
                            .uses
                            .insert(VirtualUse::Alias { ip, slot });
                    }
                } else {
                    self.escape_value(&value, EscapeReason::UnsafeSlot { ip, slot });
                    if let Some(target) = state.locals.get_mut(slot) {
                        *target = AbstractValue::unknown();
                    }
                }
            }
            Op::ClearLocal(slot) => {
                let target = state
                    .locals
                    .get_mut(slot)
                    .ok_or(AnalysisFailure::InvalidStack(ip))?;
                *target = AbstractValue::known_non_function();
            }
            Op::DefineGlobalVar(_)
            | Op::StoreGlobalStrict(_)
            | Op::StoreGlobalSloppy { .. }
            | Op::StoreLocalOrGlobalSloppy { .. } => {
                let value = Self::pop(state, ip)?;
                self.escape_value(&value, EscapeReason::IdentityUse { ip });
            }
            Op::Pop => {
                let value = Self::pop(state, ip)?;
                for candidate in &value.candidates {
                    self.candidates[*candidate]
                        .uses
                        .insert(VirtualUse::Discard { ip });
                }
            }
            Op::Dup => {
                let value = state
                    .stack
                    .last()
                    .cloned()
                    .ok_or(AnalysisFailure::InvalidStack(ip))?;
                state.stack.push(value);
            }
            Op::NewArray { elements } => {
                let value_count = elements
                    .iter()
                    .filter(|element| !matches!(element, ArrayElementKind::Elision))
                    .count();
                let inputs = Self::pop_many(state, value_count, ip)?;
                for input in &inputs {
                    self.escape_value(input, EscapeReason::StoredInAggregate { ip });
                }
                if let Some(candidate) = self.candidate_at[ip] {
                    self.candidates[candidate].seen = true;
                    state
                        .stack
                        .push(AbstractValue::virtual_candidate(candidate));
                } else {
                    state.stack.push(AbstractValue::known_non_function());
                }
            }
            Op::NewTemplateObject { .. } | Op::NewObjectLiteral => {
                state.stack.push(AbstractValue::known_non_function());
            }
            Op::NewObjectDataLiteral { shape } => {
                let inputs = Self::pop_many(state, shape.input_len(), ip)?;
                let candidate = self.candidate_at[ip]
                    .expect("object data literal should have a registered candidate");
                self.candidates[candidate].seen = true;
                for input in &inputs {
                    self.escape_value(input, EscapeReason::StoredInAggregate { ip });
                    if input.may_be_home_function {
                        self.candidates[candidate]
                            .escape_reasons
                            .insert(EscapeReason::HomeObjectSideEffect { ip });
                    }
                }
                state
                    .stack
                    .push(AbstractValue::virtual_candidate(candidate));
            }
            Op::GetPropNamed { key, cache } => {
                let receiver = if let Some(slot) = cache.local_slot() {
                    if self.authority.is_authoritative(slot) {
                        state.locals.get(slot).cloned()
                    } else {
                        Some(AbstractValue::unknown())
                    }
                    .ok_or(AnalysisFailure::InvalidStack(ip))?
                } else {
                    Self::pop(state, ip)?
                };
                let value = self.read_named(ip, &receiver, &key);
                state.stack.push(value);
            }
            Op::GetPropIndex(encoded) => {
                let (index, local_slot) = decode_index_receiver(encoded);
                let receiver = if let Some(slot) = local_slot {
                    if self.authority.is_authoritative(slot) {
                        state.locals.get(slot).cloned()
                    } else {
                        Some(AbstractValue::unknown())
                    }
                    .ok_or(AnalysisFailure::InvalidStack(ip))?
                } else {
                    Self::pop(state, ip)?
                };
                let value = self.read_index(ip, &receiver, index);
                state.stack.push(value);
            }
            Op::SetPropNamed { key, .. } => {
                let value = Self::pop(state, ip)?;
                let receiver = Self::pop(state, ip)?;
                self.write_named(ip, &receiver, &key);
                state.stack.push(value);
            }
            Op::GetProp => {
                let key = Self::pop(state, ip)?;
                let receiver = Self::pop(state, ip)?;
                self.escape_value(&key, EscapeReason::IdentityUse { ip });
                let value = if let Some(key) = key.known_string.as_deref() {
                    self.read_named(ip, &receiver, key)
                } else {
                    self.escape_value(&receiver, EscapeReason::UnknownProperty { ip });
                    AbstractValue::unknown()
                };
                state.stack.push(value);
            }
            Op::SetProp { .. } => {
                let value = Self::pop(state, ip)?;
                let key = Self::pop(state, ip)?;
                let receiver = Self::pop(state, ip)?;
                self.escape_value(&value, EscapeReason::StoredInAggregate { ip });
                self.escape_value(&key, EscapeReason::IdentityUse { ip });
                if let Some(key) = key.known_string.as_deref() {
                    self.write_named(ip, &receiver, key);
                } else {
                    self.escape_value(&receiver, EscapeReason::UnknownProperty { ip });
                }
                state.stack.push(value);
            }
            Op::ToPropertyKey | Op::ToPropertyKeyForAccess => {
                let value = Self::pop(state, ip)?;
                self.escape_value(&value, EscapeReason::IdentityUse { ip });
                state.stack.push(match value.known_string {
                    Some(value) => AbstractValue::known_string(value),
                    None => AbstractValue::known_non_function(),
                });
            }
            Op::Typeof | Op::ToString | Op::ToNumeric | Op::Unary(_) | Op::Update(_) => {
                let value = Self::pop(state, ip)?;
                self.escape_value(&value, EscapeReason::IdentityUse { ip });
                state.stack.push(AbstractValue::known_non_function());
            }
            Op::Binary(_) => {
                let right = Self::pop(state, ip)?;
                let left = Self::pop(state, ip)?;
                self.escape_value(&left, EscapeReason::IdentityUse { ip });
                self.escape_value(&right, EscapeReason::IdentityUse { ip });
                state.stack.push(AbstractValue::known_non_function());
            }
            Op::Call(argc) | Op::New(argc) => {
                self.consume_escape(ip, state, argc + 1)?;
                state.stack.push(AbstractValue::unknown());
            }
            Op::CallResolved(argc) => {
                self.consume_escape(ip, state, argc + 2)?;
                state.stack.push(AbstractValue::unknown());
            }
            Op::CallSpread | Op::NewSpread => {
                self.consume_escape(ip, state, 2)?;
                state.stack.push(AbstractValue::unknown());
            }
            Op::CallResolvedSpread => {
                self.consume_escape(ip, state, 3)?;
                state.stack.push(AbstractValue::unknown());
            }
            Op::NewFunction { .. } => state.stack.push(AbstractValue::known_function()),
            Op::RequireObjectCoercible => {
                let value = state
                    .stack
                    .last()
                    .cloned()
                    .ok_or(AnalysisFailure::InvalidStack(ip))?;
                if let Some(candidate) = value.exact_candidate() {
                    self.candidates[candidate]
                        .uses
                        .insert(VirtualUse::ObjectGuard { ip });
                } else if !value.candidates.is_empty() {
                    self.escape_value(&value, EscapeReason::AmbiguousAlias { ip });
                }
            }
            Op::RequireCallable => {
                let value = state
                    .stack
                    .last()
                    .cloned()
                    .ok_or(AnalysisFailure::InvalidStack(ip))?;
                self.escape_value(&value, EscapeReason::IdentityUse { ip });
            }
            Op::GetPrivate(_) => {
                let receiver = Self::pop(state, ip)?;
                self.escape_value(&receiver, EscapeReason::IdentityUse { ip });
                state.stack.push(AbstractValue::unknown());
            }
            Op::SetPrivate(_) => {
                let value = Self::pop(state, ip)?;
                let receiver = Self::pop(state, ip)?;
                self.escape_value(&receiver, EscapeReason::IdentityUse { ip });
                self.escape_value(&value, EscapeReason::StoredInAggregate { ip });
                state.stack.push(value);
            }
            Op::PrivateIn(_) => {
                let receiver = Self::pop(state, ip)?;
                self.escape_value(&receiver, EscapeReason::IdentityUse { ip });
                state.stack.push(AbstractValue::known_non_function());
            }
            Op::DeleteProp { .. } => {
                let key = Self::pop(state, ip)?;
                let receiver = Self::pop(state, ip)?;
                self.escape_value(&key, EscapeReason::IdentityUse { ip });
                self.escape_value(&receiver, EscapeReason::IdentityUse { ip });
                state.stack.push(AbstractValue::known_non_function());
            }
            Op::DeleteIdent(_) => state.stack.push(AbstractValue::known_non_function()),
            Op::Jump(_) => {}
            Op::JumpIfFalse(_) | Op::JumpIfTrue(_) | Op::JumpIfNotNullish(_) => {
                let condition = state
                    .stack
                    .last()
                    .cloned()
                    .ok_or(AnalysisFailure::InvalidStack(ip))?;
                self.escape_value(&condition, EscapeReason::IdentityUse { ip });
            }
            Op::FreshIterationScope(slots) => {
                for slot in slots {
                    if let Some(value) = state.locals.get(slot).cloned() {
                        self.escape_value(&value, EscapeReason::UnsafeSlot { ip, slot });
                    }
                }
            }
            Op::Return | Op::Throw => {
                if let Some(value) = state.stack.pop() {
                    self.escape_value(&value, EscapeReason::IdentityUse { ip });
                }
            }
            Op::Yield | Op::Await => {
                self.escape_state(state, EscapeReason::Suspension { ip });
                for value in &mut state.stack {
                    *value = AbstractValue::unknown();
                }
                for value in &mut state.locals {
                    *value = AbstractValue::unknown();
                }
            }
            Op::FunctionPrologueEnd | Op::ThrowReferenceError(_) => {}
            // Dynamic scope is rejected before the fixed-point walk. These
            // arms keep the match explicit if that policy is ever relaxed.
            Op::CallDirectEval { .. }
            | Op::CallDirectEvalSpread { .. }
            | Op::EnterWith
            | Op::ExitWith
            | Op::LoadIdentWith { .. }
            | Op::ResolveIdentWith { .. }
            | Op::LoadResolvedIdentWith { .. }
            | Op::StoreIdentWith { .. }
            | Op::StoreResolvedIdentWith { .. }
            | Op::TypeofIdentWith { .. }
            | Op::DeleteIdentWith { .. }
            | Op::AbruptJump(_)
            | Op::EnterTry { .. }
            | Op::ExitTry
            | Op::EndFinally
            | Op::DiscardPendingAbrupt
            | Op::NewClass { .. }
            | Op::SuperGet { .. }
            | Op::SuperReference
            | Op::SuperGetComputed
            | Op::SuperSet { .. }
            | Op::SuperSetComputed { .. }
            | Op::SuperMethod { .. }
            | Op::SuperMethodComputed
            | Op::SuperCall(_)
            | Op::SuperCallSpread
            | Op::EnterDisposableScope
            | Op::RegisterDisposable
            | Op::RegisterAsyncDisposable
            | Op::DisposeScope { .. }
            | Op::SetComputedFunctionName(_)
            | Op::DefineObjectProperty(_)
            | Op::CopyObjectSpread
            | Op::EnumerateKeys
            | Op::ForInKeyIsEnumerable
            | Op::GetIterator
            | Op::GetAsyncIterator
            | Op::AsyncIteratorComplete { .. }
            | Op::IteratorStep { .. }
            | Op::IteratorRest { .. }
            | Op::ObjectRestExcluding { .. }
            | Op::IteratorClose { .. }
            | Op::YieldDelegate { .. }
            | Op::ImportCall { .. }
            | Op::ImportMeta => return Err(AnalysisFailure::Unsupported(ip)),
        }
        Ok(())
    }

    fn consume_escape(
        &mut self,
        ip: usize,
        state: &mut FlowState,
        count: usize,
    ) -> Result<(), AnalysisFailure> {
        for value in Self::pop_many(state, count, ip)? {
            self.escape_value(&value, EscapeReason::IdentityUse { ip });
        }
        Ok(())
    }

    fn read_named(&mut self, ip: usize, receiver: &AbstractValue, key: &str) -> AbstractValue {
        let Some(candidate) = receiver.exact_candidate() else {
            if !receiver.candidates.is_empty() {
                self.escape_value(receiver, EscapeReason::AmbiguousAlias { ip });
            }
            return AbstractValue::unknown();
        };
        let use_kind = match &self.candidates[candidate].kind {
            VirtualKind::Object(shape) => shape
                .final_input_index(key)
                .map(|input| VirtualUse::FieldRead { ip, input }),
            VirtualKind::DenseArray { .. } if key == "length" => {
                Some(VirtualUse::ArrayLengthRead { ip })
            }
            VirtualKind::DenseArray { .. } => None,
        };
        if let Some(use_kind) = use_kind {
            self.candidates[candidate].uses.insert(use_kind);
            AbstractValue::known_non_function()
        } else {
            self.candidates[candidate]
                .escape_reasons
                .insert(EscapeReason::UnknownProperty { ip });
            AbstractValue::unknown()
        }
    }

    fn read_index(&mut self, ip: usize, receiver: &AbstractValue, index: usize) -> AbstractValue {
        let Some(candidate) = receiver.exact_candidate() else {
            if !receiver.candidates.is_empty() {
                self.escape_value(receiver, EscapeReason::AmbiguousAlias { ip });
            }
            return AbstractValue::unknown();
        };
        let use_kind = match &self.candidates[candidate].kind {
            VirtualKind::Object(shape) => shape
                .final_input_index(&index.to_string())
                .map(|input| VirtualUse::FieldRead { ip, input }),
            VirtualKind::DenseArray { length } if index < *length => {
                Some(VirtualUse::ElementRead { ip, index })
            }
            VirtualKind::DenseArray { .. } => None,
        };
        if let Some(use_kind) = use_kind {
            self.candidates[candidate].uses.insert(use_kind);
            AbstractValue::unknown()
        } else {
            self.candidates[candidate]
                .escape_reasons
                .insert(EscapeReason::UnknownProperty { ip });
            AbstractValue::unknown()
        }
    }

    fn write_named(&mut self, ip: usize, receiver: &AbstractValue, key: &str) {
        let Some(candidate) = receiver.exact_candidate() else {
            if !receiver.candidates.is_empty() {
                self.escape_value(receiver, EscapeReason::AmbiguousAlias { ip });
            }
            return;
        };
        let input = match &self.candidates[candidate].kind {
            VirtualKind::Object(shape) => shape.final_input_index(key),
            VirtualKind::DenseArray { .. } => None,
        };
        if let Some(input) = input {
            self.candidates[candidate]
                .uses
                .insert(VirtualUse::FieldWrite { ip, input });
        } else {
            self.candidates[candidate]
                .escape_reasons
                .insert(EscapeReason::UnknownProperty { ip });
        }
    }
}

fn decode_index_receiver(encoded_index: usize) -> (usize, Option<usize>) {
    if usize::BITS > u32::BITS {
        let encoded_slot = encoded_index >> u32::BITS;
        (
            encoded_index & u32::MAX as usize,
            encoded_slot.checked_sub(1),
        )
    } else {
        (encoded_index, None)
    }
}

pub(super) fn analyze(bytecode: &Bytecode) -> VirtualObjectAnalysis {
    let cfg = match ControlFlowGraph::build(&bytecode.code) {
        Ok(cfg) => cfg,
        Err(()) => {
            let mut analyzer = Analyzer::new(bytecode);
            analyzer.mark_all(EscapeReason::InvalidControlFlow);
            return VirtualObjectAnalysis {
                cfg: ControlFlowGraph {
                    blocks: Vec::new(),
                    instruction_blocks: Vec::new(),
                },
                slot_authority: analyzer.authority,
                candidates: analyzer.candidates,
                complete: false,
            };
        }
    };
    let mut analyzer = Analyzer::new(bytecode);
    if analyzer.authority.dynamic_scope {
        analyzer.mark_all(EscapeReason::DynamicScope);
        return VirtualObjectAnalysis {
            cfg,
            slot_authority: analyzer.authority,
            candidates: analyzer.candidates,
            complete: true,
        };
    }
    if bytecode.code.iter().any(|op| {
        matches!(
            op,
            Op::AbruptJump(_)
                | Op::EnterTry { .. }
                | Op::ExitTry
                | Op::EndFinally
                | Op::DiscardPendingAbrupt
        )
    }) {
        analyzer.mark_all(EscapeReason::UnsupportedInstruction { ip: 0 });
        return VirtualObjectAnalysis {
            cfg,
            slot_authority: analyzer.authority,
            candidates: analyzer.candidates,
            complete: false,
        };
    }
    if cfg.blocks.is_empty() {
        return VirtualObjectAnalysis {
            cfg,
            slot_authority: analyzer.authority,
            candidates: analyzer.candidates,
            complete: true,
        };
    }

    let mut incoming = vec![None::<FlowState>; cfg.blocks.len()];
    incoming[0] = Some(FlowState::entry(bytecode.locals.len()));
    let mut queue = VecDeque::from([0]);
    let mut queued = vec![false; cfg.blocks.len()];
    queued[0] = true;
    let failure = 'work: loop {
        let Some(block) = queue.pop_front() else {
            break None;
        };
        queued[block] = false;
        let Some(mut state) = incoming[block].clone() else {
            continue;
        };
        for ip in cfg.blocks[block].start..cfg.blocks[block].end {
            if let Err(failure) = analyzer.transfer(ip, &mut state) {
                break 'work Some(failure);
            }
        }
        for successor in &cfg.blocks[block].successors {
            let next = match &incoming[*successor] {
                Some(previous) => match previous.join(&state, cfg.blocks[*successor].start) {
                    Ok(next) => next,
                    Err(failure) => break 'work Some(failure),
                },
                None => state.clone(),
            };
            if incoming[*successor].as_ref() != Some(&next) {
                incoming[*successor] = Some(next);
                if !queued[*successor] {
                    queued[*successor] = true;
                    queue.push_back(*successor);
                }
            }
        }
    };

    let complete = failure.is_none();
    if let Some(failure) = failure {
        match failure {
            AnalysisFailure::InvalidStack(ip) => {
                analyzer.mark_all(EscapeReason::InvalidStack { ip });
            }
            AnalysisFailure::Unsupported(ip) => {
                analyzer.mark_all(EscapeReason::UnsupportedInstruction { ip });
            }
        }
    }
    for candidate in &mut analyzer.candidates {
        if !candidate.seen {
            candidate.escape_reasons.insert(EscapeReason::Unreachable);
        }
    }
    VirtualObjectAnalysis {
        cfg,
        slot_authority: analyzer.authority,
        candidates: analyzer.candidates,
        complete,
    }
}

#[cfg(test)]
mod tests {
    use qjs_parser::parse_script;

    use super::*;
    use crate::bytecode::compile_script;

    fn compile(source: &str) -> Bytecode {
        let script = parse_script(source).expect("test source should parse");
        compile_script(&script).expect("test source should compile")
    }

    fn named_function(source: &str, expected_name: &str) -> Rc<Bytecode> {
        fn find(bytecode: &Bytecode, expected_name: &str) -> Option<Rc<Bytecode>> {
            bytecode.code.iter().find_map(|op| match op {
                Op::NewFunction {
                    name: Some(name),
                    bytecode,
                    ..
                } if name == expected_name => Some(bytecode.clone()),
                Op::NewFunction { bytecode, .. } => find(bytecode, expected_name),
                _ => None,
            })
        }
        let bytecode = compile(source);
        find(&bytecode, expected_name).expect("named function bytecode should exist")
    }

    fn object_candidates(analysis: &VirtualObjectAnalysis) -> Vec<&VirtualCandidate> {
        analysis
            .candidates
            .iter()
            .filter(|candidate| matches!(candidate.kind, VirtualKind::Object(_)))
            .collect()
    }

    #[test]
    fn cfg_builds_if_loop_edges_without_instruction_offsets() {
        let bytecode = named_function(
            r#"
            function flow(flag, count) {
                var total = 0;
                while (count > 0) {
                    if (flag) total += count * 2;
                    else total -= count - 1;
                    count--;
                }
                return total;
            }
            "#,
            "flow",
        );
        let cfg = ControlFlowGraph::build(&bytecode.code).expect("valid CFG");

        assert!(cfg.blocks.len() >= 6);
        assert!(cfg.blocks.iter().any(|block| block.successors.len() == 2));
        assert!(cfg.blocks.iter().any(|block| {
            block
                .successors
                .iter()
                .any(|successor| cfg.blocks[*successor].start <= block.start)
        }));
    }

    #[test]
    fn follows_one_literal_through_loop_if_and_local_aliases() {
        let bytecode = named_function(
            r#"
            function projected(flag, count) {
                var total = 0;
                while (count > 0) {
                    var point = { x: count * 2, y: count - 1 };
                    var alias = point;
                    if (flag) total += alias.x;
                    else total += alias.y;
                    count--;
                }
                return total;
            }
            "#,
            "projected",
        );
        let analysis = analyze(&bytecode);
        let objects = object_candidates(&analysis);

        assert!(analysis.complete);
        assert_eq!(objects.len(), 1);
        assert!(objects[0].is_virtualizable());
        assert!(
            objects[0]
                .uses
                .iter()
                .any(|use_kind| matches!(use_kind, VirtualUse::Alias { .. }))
        );
        assert_eq!(
            objects[0]
                .uses
                .iter()
                .filter(|use_kind| matches!(use_kind, VirtualUse::FieldRead { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn join_with_non_literal_alias_fails_closed() {
        let bytecode = named_function(
            r#"
            function joined(flag, other) {
                var point;
                if (flag) point = { x: 1, y: 2 };
                else point = other;
                return point.x;
            }
            "#,
            "joined",
        );
        let analysis = analyze(&bytecode);
        let objects = object_candidates(&analysis);

        assert!(analysis.complete);
        assert_eq!(objects.len(), 1);
        assert!(!objects[0].is_virtualizable());
        assert!(
            objects[0]
                .escape_reasons
                .iter()
                .any(|reason| matches!(reason, EscapeReason::AmbiguousAlias { .. }))
        );
    }

    #[test]
    fn return_and_call_identity_uses_escape() {
        for source in [
            r#"function escaped() { var point = { x: 1 }; return point; }"#,
            r#"function escaped(sink) { var point = { x: 1 }; sink(point); return 0; }"#,
        ] {
            let bytecode = named_function(source, "escaped");
            let analysis = analyze(&bytecode);
            let objects = object_candidates(&analysis);

            assert_eq!(objects.len(), 1);
            assert!(!objects[0].is_virtualizable());
            assert!(
                objects[0]
                    .escape_reasons
                    .iter()
                    .any(|reason| matches!(reason, EscapeReason::IdentityUse { .. }))
            );
        }
    }

    #[test]
    fn dynamic_eval_and_with_reject_candidates() {
        for source in [
            r#"function dynamic() { eval(""); var point = { x: 1 }; return point.x; }"#,
            r#"function dynamic(scope) { with (scope) { var point = { x: 1 }; return point.x; } }"#,
        ] {
            let bytecode = named_function(source, "dynamic");
            let analysis = analyze(&bytecode);
            let objects = object_candidates(&analysis);

            assert_eq!(objects.len(), 1);
            assert!(!objects[0].is_virtualizable());
            assert!(
                objects[0]
                    .escape_reasons
                    .contains(&EscapeReason::DynamicScope)
            );
        }
    }

    #[test]
    fn captured_and_parameter_slots_are_not_authoritative() {
        let captured = named_function(
            r#"
            function captured() {
                var point = { x: 1 };
                return function read() { return point.x; };
            }
            "#,
            "captured",
        );
        let captured_analysis = analyze(&captured);
        let captured_objects = object_candidates(&captured_analysis);
        assert_eq!(captured_objects.len(), 1);
        assert!(!captured_objects[0].is_virtualizable());
        assert!(
            captured_objects[0]
                .escape_reasons
                .iter()
                .any(|reason| matches!(reason, EscapeReason::UnsafeSlot { .. }))
        );

        let parameter = named_function(
            r#"
            function mapped(point) {
                point = { x: 1 };
                return arguments[0];
            }
            "#,
            "mapped",
        );
        let parameter_analysis = analyze(&parameter);
        let parameter_objects = object_candidates(&parameter_analysis);
        let point_slot = parameter.local_slot("point").expect("point slot");
        assert!(
            !parameter_analysis
                .slot_authority
                .is_authoritative(point_slot)
        );
        assert_eq!(parameter_objects.len(), 1);
        assert!(!parameter_objects[0].is_virtualizable());
        assert!(
            parameter_objects[0]
                .escape_reasons
                .contains(&EscapeReason::UnsafeSlot {
                    ip: parameter_objects[0]
                        .escape_reasons
                        .iter()
                        .find_map(|reason| match reason {
                            EscapeReason::UnsafeSlot { ip, slot } if *slot == point_slot =>
                                Some(*ip),
                            _ => None,
                        })
                        .expect("parameter store should be rejected"),
                    slot: point_slot,
                })
        );
    }

    #[test]
    fn duplicate_keys_project_the_last_input() {
        let shape = ObjectLiteralShape::new(vec![Rc::from("x"), Rc::from("y"), Rc::from("x")]);
        assert_eq!(shape.final_input_index("x"), Some(2));
        assert_eq!(shape.final_input_index("y"), Some(1));
        assert_eq!(shape.final_input_index("missing"), None);

        let bytecode = named_function(
            r#"function duplicate() { var point = { x: 1, y: 2, x: 3 }; return point.x; }"#,
            "duplicate",
        );
        let analysis = analyze(&bytecode);
        let objects = object_candidates(&analysis);
        assert_eq!(objects.len(), 1);
        assert!(objects[0].is_virtualizable());
        assert!(
            objects[0]
                .uses
                .iter()
                .any(|use_kind| matches!(use_kind, VirtualUse::FieldRead { input: 2, .. }))
        );
    }

    #[test]
    fn kraken_freqz_shape_is_a_general_read_write_candidate() {
        // Reduced from Kraken 1.1 DSP.freqz: the literal/update/read topology
        // is preserved, while corpus data and benchmark harness code are not
        // copied into this non-benchmark unit fixture.
        let bytecode = named_function(
            r#"
            function freqz(b, a, w, cos, sin, sqrt) {
                var result = [];
                for (var i = 0; i < w.length; i++) {
                    var numerator = { real: 0.0, imag: 0.0 };
                    for (var j = 0; j < b.length; j++) {
                        numerator.real += b[j] * cos(-j * w[i]);
                        numerator.imag += b[j] * sin(-j * w[i]);
                    }
                    var denominator = { real: 0.0, imag: 0.0 };
                    for (var k = 0; k < a.length; k++) {
                        denominator.real += a[k] * cos(-k * w[i]);
                        denominator.imag += a[k] * sin(-k * w[i]);
                    }
                    result[i] = sqrt(
                        numerator.real * numerator.real + numerator.imag * numerator.imag
                    ) / sqrt(
                        denominator.real * denominator.real + denominator.imag * denominator.imag
                    );
                }
                return result;
            }
            "#,
            "freqz",
        );
        let analysis = analyze(&bytecode);
        let objects = object_candidates(&analysis);

        assert!(analysis.complete);
        assert_eq!(objects.len(), 2);
        for candidate in objects {
            assert!(candidate.is_virtualizable(), "{candidate:#?}");
            assert!(
                candidate
                    .uses
                    .iter()
                    .any(|use_kind| matches!(use_kind, VirtualUse::FieldWrite { .. }))
            );
            assert!(
                candidate
                    .uses
                    .iter()
                    .any(|use_kind| matches!(use_kind, VirtualUse::FieldRead { .. }))
            );
        }
    }
}
