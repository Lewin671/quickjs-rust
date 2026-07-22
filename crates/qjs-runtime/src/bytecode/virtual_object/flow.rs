use std::{
    collections::{BTreeSet, VecDeque},
    rc::Rc,
};

use crate::Value;

use super::super::ir::{ArrayElementKind, Bytecode, Op, decode_index_receiver};
use super::{
    CandidateId, EscapeReason, SlotAuthority, VirtualCandidate, VirtualKind, VirtualObjectAnalysis,
    VirtualUse, cfg::ControlFlowGraph,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum StringKnowledge {
    Exact(Rc<String>),
    SomeString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AbstractValue {
    candidates: BTreeSet<CandidateId>,
    may_be_other: bool,
    may_be_home_function: bool,
    known_string: Option<StringKnowledge>,
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
            known_string: Some(StringKnowledge::Exact(value)),
        }
    }

    fn some_string() -> Self {
        Self {
            candidates: BTreeSet::new(),
            may_be_other: true,
            may_be_home_function: false,
            known_string: Some(StringKnowledge::SomeString),
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

    fn exact_string(&self) -> Option<&str> {
        match &self.known_string {
            Some(StringKnowledge::Exact(value)) => Some(value),
            Some(StringKnowledge::SomeString) | None => None,
        }
    }

    fn append_literal(&self, suffix: &str) -> Self {
        let Some(prefix) = self.exact_string() else {
            return Self::some_string();
        };
        let mut value = String::with_capacity(prefix.len() + suffix.len());
        value.push_str(prefix);
        value.push_str(suffix);
        Self::known_string(Rc::new(value))
    }

    fn join(&self, other: &Self) -> Self {
        let mut candidates = self.candidates.clone();
        candidates.extend(other.candidates.iter().copied());
        Self {
            candidates,
            may_be_other: self.may_be_other || other.may_be_other,
            may_be_home_function: self.may_be_home_function || other.may_be_home_function,
            known_string: match (&self.known_string, &other.known_string) {
                (Some(left), Some(right)) if left == right => Some(left.clone()),
                (Some(_), Some(_)) => Some(StringKnowledge::SomeString),
                _ => None,
            },
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

    fn store_alias(
        &mut self,
        ip: usize,
        state: &mut FlowState,
        slot: usize,
        value: AbstractValue,
        authoritative: bool,
    ) -> Result<(), AnalysisFailure> {
        if authoritative {
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
        Ok(())
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
            Op::AppendStringLiteralLocal { slot, value } => {
                let stored = state
                    .locals
                    .get(slot)
                    .cloned()
                    .ok_or(AnalysisFailure::InvalidStack(ip))?;
                let previous = if self.authority.is_authoritative(slot) {
                    stored
                } else {
                    AbstractValue::unknown()
                };
                self.escape_value(&previous, EscapeReason::IdentityUse { ip });
                let result = previous.append_literal(&value);
                state.locals[slot] = if self.authority.is_authoritative(slot) {
                    result.clone()
                } else {
                    AbstractValue::unknown()
                };
                state.stack.push(result);
            }
            Op::AppendStringLiteralGlobal { .. } => {
                // A completed `x += "literal"` always produces a string even
                // though a global binding's prefix is not tracked here.
                state.stack.push(AbstractValue::some_string());
            }
            Op::StoreLocal(slot) => {
                let value = Self::pop(state, ip)?;
                let authoritative = self.authority.is_authoritative(slot);
                self.store_alias(ip, state, slot, value, authoritative)?;
            }
            Op::AssignLocal(slot) => {
                let value = Self::pop(state, ip)?;
                let authoritative = self
                    .authority
                    .is_assignment_authoritative(self.bytecode, slot);
                self.store_alias(ip, state, slot, value, authoritative)?;
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
                self.store_named_value(ip, &receiver, &value, &key);
                state.stack.push(value);
            }
            Op::SetPropIndex { index, .. } => {
                let value = Self::pop(state, ip)?;
                let receiver = Self::pop(state, ip)?;
                self.store_indexed_value(ip, &receiver, &value, index);
                state.stack.push(value);
            }
            Op::GetProp => {
                let key = Self::pop(state, ip)?;
                let receiver = Self::pop(state, ip)?;
                self.escape_value(&key, EscapeReason::IdentityUse { ip });
                let value = if let Some(key) = key.exact_string() {
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
                self.escape_value(&key, EscapeReason::IdentityUse { ip });
                if let Some(key) = key.exact_string() {
                    if let Some(index) = canonical_array_index(key) {
                        self.store_indexed_value(ip, &receiver, &value, index);
                    } else {
                        self.store_named_value(ip, &receiver, &value, key);
                    }
                } else {
                    self.escape_value(&value, EscapeReason::StoredInAggregate { ip });
                    self.escape_value(&receiver, EscapeReason::UnknownProperty { ip });
                }
                state.stack.push(value);
            }
            Op::ToPropertyKey | Op::ToPropertyKeyForAccess => {
                let value = Self::pop(state, ip)?;
                self.escape_value(&value, EscapeReason::IdentityUse { ip });
                state.stack.push(match value.known_string {
                    Some(value) => AbstractValue {
                        candidates: BTreeSet::new(),
                        may_be_other: true,
                        may_be_home_function: false,
                        known_string: Some(value),
                    },
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
            | Op::ImportMeta
            | Op::InitVirtualObject { .. }
            | Op::LoadVirtualValue { .. }
            | Op::StoreVirtualValue { .. }
            | Op::LoadVirtualLength { .. }
            | Op::GuardVirtualObject
            | Op::InitVirtualConstants { .. }
            | Op::LoadVirtualBinary { .. }
            | Op::BinaryAssignLocals { .. }
            | Op::IncrementLocal { .. }
            | Op::CopyLocal { .. }
            | Op::CompareLocalsJumpFalse { .. } => {
                return Err(AnalysisFailure::Unsupported(ip));
            }
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
            let is_array_length = matches!(use_kind, VirtualUse::ArrayLengthRead { .. });
            self.candidates[candidate].uses.insert(use_kind);
            if is_array_length {
                AbstractValue::known_non_function()
            } else {
                // The analysis does not yet maintain a field-value lattice.
                // A data field may hold a function whose later object-literal
                // insertion performs SetFunctionName/home-object work, so a
                // field read must retain that possibility.
                AbstractValue::unknown()
            }
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
        match &self.candidates[candidate].kind {
            VirtualKind::Object(shape) => {
                if let Some(input) = shape.final_input_index(key) {
                    self.candidates[candidate]
                        .uses
                        .insert(VirtualUse::FieldWrite { ip, input });
                } else {
                    self.candidates[candidate]
                        .escape_reasons
                        .insert(EscapeReason::UnknownProperty { ip });
                }
            }
            VirtualKind::DenseArray { length } => {
                let Some(index) = canonical_array_index(key).filter(|index| index < length) else {
                    self.candidates[candidate]
                        .escape_reasons
                        .insert(EscapeReason::UnknownProperty { ip });
                    return;
                };
                self.candidates[candidate]
                    .uses
                    .insert(VirtualUse::ElementWrite { ip, index });
            }
        }
    }

    fn write_index(&mut self, ip: usize, receiver: &AbstractValue, index: usize) {
        let Some(candidate) = receiver.exact_candidate() else {
            if !receiver.candidates.is_empty() {
                self.escape_value(receiver, EscapeReason::AmbiguousAlias { ip });
            }
            return;
        };
        let use_kind = match &self.candidates[candidate].kind {
            VirtualKind::Object(shape) => shape
                .final_input_index(&index.to_string())
                .map(|input| VirtualUse::FieldWrite { ip, input }),
            VirtualKind::DenseArray { length } if index < *length => {
                Some(VirtualUse::ElementWrite { ip, index })
            }
            VirtualKind::DenseArray { .. } => None,
        };
        if let Some(use_kind) = use_kind {
            self.candidates[candidate].uses.insert(use_kind);
        } else {
            self.candidates[candidate]
                .escape_reasons
                .insert(EscapeReason::UnknownProperty { ip });
        }
    }

    fn store_named_value(
        &mut self,
        ip: usize,
        receiver: &AbstractValue,
        value: &AbstractValue,
        key: &str,
    ) {
        // The RHS becomes observably reachable through the receiver even when
        // the receiver itself is unknown. This also rejects self cycles.
        self.escape_value(value, EscapeReason::StoredInAggregate { ip });
        self.write_named(ip, receiver, key);
    }

    /// Shared transfer contract for the `SetPropIndex` operation arriving on
    /// the dense-array mutation branch. Generic computed writes already route
    /// exact canonical indices through this helper; after rebase the dedicated
    /// opcode must pop `[receiver, value]`, call this helper, and push `value`.
    fn store_indexed_value(
        &mut self,
        ip: usize,
        receiver: &AbstractValue,
        value: &AbstractValue,
        index: usize,
    ) {
        self.escape_value(value, EscapeReason::StoredInAggregate { ip });
        self.write_index(ip, receiver, index);
    }
}

fn canonical_array_index(key: &str) -> Option<usize> {
    let index = key.parse::<usize>().ok()?;
    (index.to_string() == key && index < u32::MAX as usize).then_some(index)
}

pub(super) fn analyze(bytecode: &Bytecode) -> VirtualObjectAnalysis {
    let cfg = match ControlFlowGraph::build(&bytecode.code) {
        Ok(cfg) => cfg,
        Err(()) => {
            let mut analyzer = Analyzer::new(bytecode);
            analyzer.mark_all(EscapeReason::InvalidControlFlow);
            return VirtualObjectAnalysis {
                cfg: ControlFlowGraph::empty(),
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
