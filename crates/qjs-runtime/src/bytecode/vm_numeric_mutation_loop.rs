use std::rc::Rc;

use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, value::OwnDataPropertyWrite};

use super::{
    ir::{Bytecode, Op},
    vm::Vm,
};

mod dense;
mod predicate_scan;

use dense::DenseNumericMutationLoopPlan;
use predicate_scan::{DenseNumericPredicateScanPlan, PredicateScanRun};

#[derive(Clone, Copy, Debug)]
enum NumericMutationOp {
    Add,
    Subtract,
}

impl NumericMutationOp {
    fn apply(self, value: f64, constant: f64) -> f64 {
        match self {
            Self::Add => value + constant,
            Self::Subtract => value - constant,
        }
    }
}

#[derive(Clone, Debug)]
struct NumericMutation {
    source: usize,
    target: usize,
    operation: NumericMutationOp,
    constant: f64,
}

/// A counted loop that scalar-replaces writable numeric fields on one ordinary
/// object. Every source iteration still performs each recurrence step; only
/// the unobservable intermediate property storage is sunk to the loop exit.
#[derive(Clone, Debug)]
struct NamedNumericMutationLoopPlan {
    counter_slot: usize,
    limit_slot: usize,
    accumulator_slot: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
    receiver_slot: usize,
    fields: Vec<Rc<str>>,
    mutations: Vec<NumericMutation>,
    checksum_field: usize,
}

/// A fail-closed numeric property-mutation accelerator compiled from immutable
/// source bytecode before virtual-object lowering.
#[derive(Clone, Debug)]
pub(super) struct NumericMutationLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    kind: NumericMutationLoopKind,
}

#[derive(Clone, Debug)]
enum NumericMutationLoopKind {
    Named(NamedNumericMutationLoopPlan),
    Dense(Rc<DenseNumericMutationLoopPlan>),
    PredicateScan(Rc<DenseNumericPredicateScanPlan>),
}

impl NumericMutationLoopPlan {
    pub(super) fn compile_all(bytecode: &Bytecode) -> Vec<Self> {
        bytecode
            .code
            .iter()
            .enumerate()
            .filter_map(|(backedge, op)| match op {
                Op::Jump(header) if *header < backedge => {
                    Self::compile(bytecode, *header, backedge)
                }
                _ => None,
            })
            .collect()
    }

    pub(super) fn contains_instruction(&self, ip: usize) -> bool {
        match &self.kind {
            NumericMutationLoopKind::PredicateScan(plan) => plan.contains_instruction(ip),
            NumericMutationLoopKind::Named(_) | NumericMutationLoopKind::Dense(_) => {
                (self.header..=self.backedge).contains(&ip)
            }
        }
    }

    fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        if let Some(named) = NamedNumericMutationLoopPlan::compile(bytecode, header, backedge) {
            return Some(Self {
                header,
                backedge,
                exit: named.exit,
                kind: NumericMutationLoopKind::Named(named.plan),
            });
        }
        if let Some(dense) = DenseNumericMutationLoopPlan::compile(bytecode, header, backedge) {
            return Some(Self {
                header,
                backedge,
                exit: dense.exit(),
                kind: NumericMutationLoopKind::Dense(Rc::new(dense)),
            });
        }
        let predicate_scan = DenseNumericPredicateScanPlan::compile(bytecode, header, backedge)?;
        Some(Self {
            header,
            backedge,
            exit: predicate_scan.exit(),
            kind: NumericMutationLoopKind::PredicateScan(Rc::new(predicate_scan)),
        })
    }

    fn try_run(&self, vm: &mut Vm<'_>) -> NumericMutationLoopRun {
        match &self.kind {
            NumericMutationLoopKind::Named(plan) => {
                NumericMutationLoopRun::from_handled(plan.try_run(vm, self.exit))
            }
            NumericMutationLoopKind::Dense(plan) => {
                NumericMutationLoopRun::from_handled(plan.try_run(vm))
            }
            NumericMutationLoopKind::PredicateScan(plan) => match plan.try_run(vm) {
                PredicateScanRun::Handled => NumericMutationLoopRun::Handled,
                PredicateScanRun::Suppress => NumericMutationLoopRun::SuppressPredicate,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumericMutationLoopRun {
    Handled,
    Declined,
    SuppressPredicate,
}

impl NumericMutationLoopRun {
    fn from_handled(handled: bool) -> Self {
        if handled {
            Self::Handled
        } else {
            Self::Declined
        }
    }
}

struct CompiledNamedPlan {
    exit: usize,
    plan: NamedNumericMutationLoopPlan,
}

impl NamedNumericMutationLoopPlan {
    fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<CompiledNamedPlan> {
        let code = &bytecode.code;
        let (
            Op::LoadLocal(counter_slot),
            Op::LoadLocal(limit_slot),
            Op::Binary(BinaryOp::Lt),
            Op::JumpIfFalse(exit),
            Op::Pop,
            Op::LoadConst(_),
            Op::StoreLocal(block_result_slot),
        ) = (
            code.get(header)?,
            code.get(header + 1)?,
            code.get(header + 2)?,
            code.get(header + 3)?,
            code.get(header + 4)?,
            code.get(header + 5)?,
            code.get(header + 6)?,
        )
        else {
            return None;
        };
        if !matches!(code.get(*exit), Some(Op::Pop)) {
            return None;
        }

        let tail = backedge.checked_sub(8)?;
        let (
            Op::LoadLocal(tail_block_result_slot),
            Op::StoreLocal(loop_result_slot),
            Op::LoadLocal(tail_counter_slot),
            Op::ToNumeric,
            Op::Dup,
            Op::Update(UpdateOp::Increment),
            Op::AssignLocal(assigned_counter_slot),
            Op::Pop,
            Op::Jump(tail_header),
        ) = (
            code.get(tail)?,
            code.get(tail + 1)?,
            code.get(tail + 2)?,
            code.get(tail + 3)?,
            code.get(tail + 4)?,
            code.get(tail + 5)?,
            code.get(tail + 6)?,
            code.get(tail + 7)?,
            code.get(tail + 8)?,
        )
        else {
            return None;
        };
        if tail + 8 != backedge
            || tail_header != &header
            || tail_block_result_slot != block_result_slot
            || tail_counter_slot != counter_slot
            || assigned_counter_slot != counter_slot
        {
            return None;
        }

        let mut cursor = header + 7;
        let mut receiver_slot = None;
        let mut fields = Vec::new();
        let mut mutations = Vec::new();
        while let Some((mutation, next, slot)) = compile_mutation(
            bytecode,
            cursor,
            *block_result_slot,
            *loop_result_slot,
            &mut fields,
        ) {
            if receiver_slot.is_some_and(|current| current != slot) {
                return None;
            }
            receiver_slot = Some(slot);
            mutations.push(mutation);
            cursor = next;
        }
        if mutations.is_empty() || mutations.len() > 8 {
            return None;
        }

        let (
            Op::LoadLocal(accumulator_slot),
            Op::GetPropNamed { key, cache },
            Op::Binary(BinaryOp::Add),
            Op::Dup,
            Op::AssignLocal(assigned_accumulator_slot),
            Op::Dup,
            Op::StoreLocal(accumulator_block_result_slot),
            Op::StoreLocal(accumulator_loop_result_slot),
        ) = (
            code.get(cursor)?,
            code.get(cursor + 1)?,
            code.get(cursor + 2)?,
            code.get(cursor + 3)?,
            code.get(cursor + 4)?,
            code.get(cursor + 5)?,
            code.get(cursor + 6)?,
            code.get(cursor + 7)?,
        )
        else {
            return None;
        };
        let receiver_slot = receiver_slot?;
        if cursor + 8 != tail
            || cache.local_slot() != Some(receiver_slot)
            || assigned_accumulator_slot != accumulator_slot
            || accumulator_block_result_slot != block_result_slot
            || accumulator_loop_result_slot != loop_result_slot
        {
            return None;
        }
        let checksum_field = field_index(&mut fields, key);

        Some(CompiledNamedPlan {
            exit: *exit,
            plan: Self {
                counter_slot: *counter_slot,
                limit_slot: *limit_slot,
                accumulator_slot: *accumulator_slot,
                block_result_slot: *block_result_slot,
                loop_result_slot: *loop_result_slot,
                receiver_slot,
                fields,
                mutations,
                checksum_field,
            },
        })
    }

    fn try_run(&self, vm: &mut Vm<'_>, exit: usize) -> bool {
        if vm.direct_eval_with_stack {
            return false;
        }
        for slot in [
            self.counter_slot,
            self.limit_slot,
            self.accumulator_slot,
            self.block_result_slot,
            self.loop_result_slot,
            self.receiver_slot,
        ] {
            if !vm.slot_is_authoritative(slot) {
                return false;
            }
        }
        let Some(mut counter) = local_number(vm, self.counter_slot) else {
            return false;
        };
        let Some(limit) = local_number(vm, self.limit_slot) else {
            return false;
        };
        let Some(mut accumulator) = local_number(vm, self.accumulator_slot) else {
            return false;
        };
        let Some(Some(Value::Object(object))) = vm.locals.get(self.receiver_slot) else {
            return false;
        };
        if vm.is_global_object(&Value::Object(object.clone()))
            || crate::symbol::is_symbol_primitive(object)
            || crate::typed_array::is_typed_array_object(object)
            || object.is_module_namespace_exotic()
        {
            return false;
        }
        let mut values = Vec::with_capacity(self.fields.len());
        for field in &self.fields {
            let Some(value) = object.writable_own_data_number(field) else {
                return false;
            };
            values.push(value);
        }
        let object = object.clone();

        while counter < limit {
            for mutation in &self.mutations {
                values[mutation.target] = mutation
                    .operation
                    .apply(values[mutation.source], mutation.constant);
            }
            accumulator += values[self.checksum_field];
            counter += 1.0;
        }
        for mutation in &self.mutations {
            let key = &self.fields[mutation.target];
            let value = Value::Number(values[mutation.target]);
            if !matches!(
                object.write_existing_own_data_property(key, &value),
                OwnDataPropertyWrite::Written
            ) {
                return false;
            }
        }

        set_local_number(vm, self.counter_slot, counter);
        set_local_number(vm, self.accumulator_slot, accumulator);
        set_local_number(vm, self.block_result_slot, accumulator);
        set_local_number(vm, self.loop_result_slot, accumulator);
        vm.ip = exit + 1;
        true
    }
}

fn compile_mutation(
    bytecode: &Bytecode,
    cursor: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
    fields: &mut Vec<Rc<str>>,
) -> Option<(NumericMutation, usize, usize)> {
    let code = &bytecode.code;
    let (
        Op::LoadLocal(receiver_slot),
        Op::GetPropNamed {
            key: source_key,
            cache,
        },
        Op::LoadConst(constant_index),
        Op::Binary(operation),
        Op::SetPropNamed {
            key: target_key, ..
        },
        Op::Dup,
        Op::StoreLocal(write_block_result_slot),
        Op::StoreLocal(write_loop_result_slot),
    ) = (
        code.get(cursor)?,
        code.get(cursor + 1)?,
        code.get(cursor + 2)?,
        code.get(cursor + 3)?,
        code.get(cursor + 4)?,
        code.get(cursor + 5)?,
        code.get(cursor + 6)?,
        code.get(cursor + 7)?,
    )
    else {
        return None;
    };
    if cache.local_slot() != Some(*receiver_slot)
        || write_block_result_slot != &block_result_slot
        || write_loop_result_slot != &loop_result_slot
    {
        return None;
    }
    let Value::Number(constant) = bytecode.constants.get(*constant_index)? else {
        return None;
    };
    let operation = match operation {
        BinaryOp::Add => NumericMutationOp::Add,
        BinaryOp::Sub => NumericMutationOp::Subtract,
        _ => return None,
    };
    let source = field_index(fields, source_key);
    let target = field_index(fields, target_key);
    Some((
        NumericMutation {
            source,
            target,
            operation,
            constant: *constant,
        },
        cursor + 8,
        *receiver_slot,
    ))
}

fn field_index(fields: &mut Vec<Rc<str>>, key: &Rc<str>) -> usize {
    if let Some(index) = fields.iter().position(|field| field == key) {
        return index;
    }
    fields.push(key.clone());
    fields.len() - 1
}

fn local_number(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    match vm.local_slot_value(slot)? {
        Value::Number(value) => Some(value),
        _ => None,
    }
}

fn set_local_number(vm: &mut Vm<'_>, slot: usize, value: f64) {
    vm.locals[slot] = Some(Value::Number(value));
}

pub(super) fn try_run_numeric_mutation_loop(
    vm: &mut Vm<'_>,
    header: usize,
    backedge: usize,
) -> bool {
    let Some((index, plan)) = vm
        .numeric_mutation_loop_plans
        .iter()
        .enumerate()
        .find(|(_, plan)| plan.header == header && plan.backedge == backedge)
        .map(|(index, plan)| (index, plan.clone()))
    else {
        return false;
    };
    match plan.try_run(vm) {
        NumericMutationLoopRun::Handled => true,
        NumericMutationLoopRun::Declined => false,
        NumericMutationLoopRun::SuppressPredicate => {
            // Plans are already cloned into each frame. Removing a zero-
            // progress predicate plan suppresses only this invocation and
            // adds no state to the call-path-sensitive FrameState layout.
            vm.numeric_mutation_loop_plans.remove(index);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;
    use crate::{Value, eval};

    fn nested_function(source: &str) -> Bytecode {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");
        bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
                _ => None,
            })
            .expect("function bytecode should be nested in the script")
    }

    #[test]
    fn recognizes_named_numeric_recurrence() {
        let bytecode = nested_function(
            "function run(n) { var o = { a: 0, b: 0, c: 0 }; var sum = 0; for (var i = 0; i < n; i++) { o.a = o.c + 1; o.b = o.a + 1; o.c = o.b - 1; sum += o.c; } return sum; }",
        );
        assert_eq!(NumericMutationLoopPlan::compile_all(&bytecode).len(), 1);
    }

    #[test]
    fn recognizes_fixed_dense_numeric_recurrence() {
        let bytecode = nested_function(
            "function run(n) { var a = [0, 0, 0, 0]; var sum = 0; for (var i = 0; i < n; i++) { a[0] = a[3] + 1; a[1] = a[0] + 1; a[2] = a[1] - 1; a[3] = a[2]; sum += a[3]; } return sum; }",
        );
        let plans = NumericMutationLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
        assert!(matches!(plans[0].kind, NumericMutationLoopKind::Dense(_)));
    }

    #[test]
    fn commits_fixed_dense_recurrence_at_loop_exit() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function run(n) { var a = [0, 0, 0, 0]; var sum = 0; for (var i = 0; i < n; i++) { a[0] = a[3] + 1; a[1] = a[0] + 1; a[2] = a[1] - 1; a[3] = a[2]; sum += a[3]; } return sum + ':' + a.join(':'); } run(1000);"
            ),
            Ok(Value::String(
                "500500:1000:1001:1000:1000".to_owned().into()
            ))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn fixed_dense_recurrence_preserves_self_and_overlapping_writes() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function run(n) { var a = [0, 0]; var sum = 0; for (var i = 0; i < n; i++) { a[0] = a[0] + 1; a[1] = a[0] + 1; a[0] = a[1] - 1; sum += a[0]; } return sum + ':' + a[0] + ':' + a[1]; } run(4);"
            ),
            Ok(Value::String("10:4:5".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn fixed_dense_recurrence_falls_back_for_non_number_elements() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function run(n) { var hits = 0; var marker = { valueOf: function () { hits += 1; return 1; } }; var a = [0, 0, marker]; var sum = 0; for (var i = 0; i < n; i++) { a[0] = a[0] + 1; a[1] = a[0] + 1; a[0] = a[1] - 1; sum += a[2]; } return sum + ':' + a[0] + ':' + a[1] + ':' + hits; } run(4);"
            ),
            Ok(Value::String("4:4:5:4".to_owned().into()))
        );
        assert_eq!(dense::test_iterations(), 0);
    }

    #[test]
    fn recognizes_dynamic_dense_numeric_rmw() {
        let bytecode = nested_function(
            "function run(n) { var a = [255, 255, 255, 255, 255, 255, 255, 255]; for (var j = 0; j < n; j = j + 2) { a[j] &= ~(1 << (j & 31)); } return a[0]; }",
        );
        let plans = NumericMutationLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
        assert!(matches!(plans[0].kind, NumericMutationLoopKind::Dense(_)));
    }

    #[test]
    fn runs_dynamic_dense_numeric_rmw() {
        assert_eq!(
            eval(
                "function run(n) { var a = [255, 255, 255, 255, 255, 255, 255, 255]; for (var j = 0; j < n; j = j + 2) { a[j] &= ~(1 << (j & 31)); } return a.join(':'); } run(8);"
            ),
            Ok(Value::String(
                "254:255:251:255:239:255:191:255".to_owned().into()
            ))
        );
    }

    #[test]
    fn read_only_dense_region_accelerates_aliased_multi_array_rounds() {
        let source = "function round(a, q, v, w, d, e, f, g, b, n, p) { var h, k, l, m; for (m = 0; m < n; m++) { h = a[e >>> 24] ^ q[(f >> 16) & 255] ^ v[(g >> 8) & 255] ^ w[b & 255] ^ d[p]; k = a[f >>> 24] ^ q[(g >> 16) & 255] ^ v[(b >> 8) & 255] ^ w[e & 255] ^ d[p + 1]; l = a[g >>> 24] ^ q[(b >> 16) & 255] ^ v[(e >> 8) & 255] ^ w[f & 255] ^ d[p + 2]; b = a[b >>> 24] ^ q[(e >> 16) & 255] ^ v[(f >> 8) & 255] ^ w[g & 255] ^ d[p + 3]; p += 4; e = h; f = k; g = l; } return e ^ f ^ g ^ b ^ p; }";
        let bytecode = nested_function(source);
        let plans = NumericMutationLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
        assert!(matches!(plans[0].kind, NumericMutationLoopKind::Dense(_)));

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var table = []; for (var i = 0; i < 256; i++) table.push(i | 0); round(table, table, table, table, table, 0, 0, 0, 0, 10, 0);"
            )),
            Ok(Value::Number(40.0))
        );
        assert_eq!(dense::test_read_only_path_hits(), 1);
        assert_eq!(dense::test_read_only_iterations(), 9);
        assert_eq!(dense::test_read_only_bailouts(), 0);
    }

    #[test]
    fn read_only_dense_region_accepts_comma_sequenced_rounds() {
        let source = "function round(a, q, v, w, d, e, f, g, b, n, p) { var h, k, l, m; for (m = 0; m < n; m++) h = a[e >>> 24] ^ q[(f >> 16) & 255] ^ v[(g >> 8) & 255] ^ w[b & 255] ^ d[p], k = a[f >>> 24] ^ q[(g >> 16) & 255] ^ v[(b >> 8) & 255] ^ w[e & 255] ^ d[p + 1], l = a[g >>> 24] ^ q[(b >> 16) & 255] ^ v[(e >> 8) & 255] ^ w[f & 255] ^ d[p + 2], b = a[b >>> 24] ^ q[(e >> 16) & 255] ^ v[(f >> 8) & 255] ^ w[g & 255] ^ d[p + 3], p += 4, e = h, f = k, g = l; return e ^ f ^ g ^ b ^ p; }";
        let bytecode = nested_function(source);
        let plans = NumericMutationLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
        assert!(matches!(plans[0].kind, NumericMutationLoopKind::Dense(_)));

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var a = [], q = [], v = [], w = [], d = []; for (var i = 0; i < 256; i++) {{ a.push(i | 0); q.push(i | 0); v.push(i | 0); w.push(i | 0); d.push(i | 0); }} round(a, q, v, w, d, 0, 0, 0, 0, 10, 0);"
            )),
            Ok(Value::Number(40.0))
        );
        assert_eq!(dense::test_read_only_path_hits(), 1);
        assert_eq!(dense::test_read_only_iterations(), 9);
        assert_eq!(dense::test_read_only_bailouts(), 0);
    }

    #[test]
    fn read_only_dense_region_hoists_direct_this_arrays_and_array_length() {
        let source = "function transform(buffer, stride) { var real = 0, imag = 0; for (var index = 0; index < buffer.length; index++) { real += this.positive[stride * index] * buffer[index]; imag += this.negative[stride * index] * buffer[index]; } return real + ':' + imag; }";
        let bytecode = nested_function(source);
        let plans = NumericMutationLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
        assert!(matches!(plans[0].kind, NumericMutationLoopKind::Dense(_)));

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var owner = {{ positive: [1,2,3,4,5,6,7,8,9,10], negative: [10,9,8,7,6,5,4,3,2,1] }}; transform.call(owner, [1,2,3,4,5,6,7,8,9,10], 1);"
            )),
            Ok(Value::String("385:220".to_owned().into()))
        );
        assert_eq!(dense::test_read_only_path_hits(), 1);
        assert_eq!(dense::test_read_only_iterations(), 9);
        assert_eq!(dense::test_read_only_bailouts(), 0);
    }

    #[test]
    fn read_only_dense_region_hoisted_sources_allow_runtime_aliases() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function transform(buffer) { var total = 0; for (var index = 0; index < buffer.length; index++) total += this.first[index] * this.second[index] * buffer[index]; return total; } var shared = [1,2,3,4,5,6,7,8,9,10]; transform.call({ first: shared, second: shared }, shared);"
            ),
            Ok(Value::Number(3025.0))
        );
        assert_eq!(dense::test_read_only_path_hits(), 1);
        assert_eq!(dense::test_read_only_iterations(), 9);
        assert_eq!(dense::test_read_only_bailouts(), 0);
    }

    #[test]
    fn read_only_dense_region_rejects_observable_hoisted_sources() {
        let source = "function transform(buffer) { var total = 0; for (var index = 0; index < buffer.length; index++) total += this.values[index] * buffer[index]; return total; }";

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var hits = 0, table = [1,2,3,4]; var owner = {{}}; Object.defineProperty(owner, 'values', {{ get: function () {{ hits++; return table; }} }}); transform.call(owner, [1,2,3,4]) + ':' + hits;"
            )),
            Ok(Value::String("30:4".to_owned().into()))
        );
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_iterations(), 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var hits = 0, table = [1,2,3,4]; var proto = {{}}; Object.defineProperty(proto, 'values', {{ get: function () {{ hits++; return table; }} }}); transform.call(Object.create(proto), [1,2,3,4]) + ':' + hits;"
            )),
            Ok(Value::String("30:4".to_owned().into()))
        );
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_iterations(), 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var hits = 0, target = {{ values: [1,2,3,4] }}; var owner = new Proxy(target, {{ get: function (object, key) {{ if (key === 'values') hits++; return object[key]; }} }}); transform.call(owner, [1,2,3,4]) + ':' + hits;"
            )),
            Ok(Value::String("30:4".to_owned().into()))
        );
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_iterations(), 0);
    }

    #[test]
    fn read_only_dense_region_rejects_discarded_direct_this_property_read() {
        let source = "function reduce(values) { var total = 0; for (var index = 0; index < values.length; index++) { this.tick; total += values[index]; } return total; }";
        let bytecode = nested_function(source);
        assert!(
            NumericMutationLoopPlan::compile_all(&bytecode).is_empty(),
            "{:#?}",
            bytecode.code
        );

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var hits = 0; var owner = {{}}; Object.defineProperty(owner, 'tick', {{ get: function () {{ hits++; return 1; }} }}); reduce.call(owner, [1,2,3,4,5,6,7,8,9,10]) + ':' + hits;"
            )),
            Ok(Value::String("55:10".to_owned().into()))
        );
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_iterations(), 0);
    }

    #[test]
    fn read_only_dense_region_rejects_observable_array_length_bound() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function reduce(values, bound) { var total = 0; for (var index = 0; index < bound.length; index++) total += values[index]; return total; } var hits = 0; var bound = {}; Object.defineProperty(bound, 'length', { get: function () { hits++; return 4; } }); reduce([1,2,3,4], bound) + ':' + hits;"
            ),
            Ok(Value::String("10:5".to_owned().into()))
        );
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_iterations(), 0);
    }

    #[test]
    fn read_only_dense_region_rejects_captured_this_and_direct_eval() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function make() { return (buffer) => { var total = 0; for (var index = 0; index < buffer.length; index++) total += this.values[index] * buffer[index]; return total; }; } var run = make.call({ values: [1,2,3,4] }); run([1,2,3,4]);"
            ),
            Ok(Value::Number(30.0))
        );
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_iterations(), 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function transform(buffer) { eval(''); var total = 0; for (var index = 0; index < buffer.length; index++) total += this.values[index] * buffer[index]; return total; } transform.call({ values: [1,2,3,4] }, [1,2,3,4]);"
            ),
            Ok(Value::Number(30.0))
        );
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_iterations(), 0);
    }

    #[test]
    fn read_only_dense_region_re_resolves_hoisted_source_after_replay() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function transform(buffer) { var total = 0; for (var index = 0; index < buffer.length; index++) total += this.values[index] * buffer[index]; return index + ':' + total; } var owner, coercions = 0, replacement = [10,20,30,40]; var marker = { valueOf: function () { coercions++; owner.values = replacement; return 2; } }; owner = { values: [1, marker, 3, 4] }; transform.call(owner, [1,2,3,4]) + ':' + coercions;"
            ),
            Ok(Value::String("4:255:1".to_owned().into()))
        );
        assert_eq!(dense::test_read_only_path_hits(), 1);
        assert_eq!(dense::test_read_only_iterations(), 2);
        assert_eq!(dense::test_read_only_bailouts(), 1);
    }

    #[test]
    fn named_array_sources_do_not_extend_writable_dense_regions() {
        let bytecode = nested_function(
            "function copy(buffer) { for (var index = 0; index < buffer.length; index++) this.values[index] = buffer[index] + 1; }",
        );
        assert!(
            NumericMutationLoopPlan::compile_all(&bytecode).is_empty(),
            "{:#?}",
            bytecode.code
        );
    }

    #[test]
    fn read_only_dense_region_rejects_captured_receiver() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function makeReduce() { var values = [1, 2, 3]; return function (bound) { var total = 0; for (var index = 0; index < bound; index++) total += values[index]; return total; }; } makeReduce()(3);"
            ),
            Ok(Value::Number(6.0))
        );
        assert_eq!(dense::test_iterations(), 0);
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_iterations(), 0);
        assert_eq!(dense::test_read_only_bailouts(), 0);
    }

    #[test]
    fn read_only_dense_region_accepts_unrelated_outer_capture() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function makeReduce() { var outer = 40; return function (values, bound) { var total = 0; for (var index = 0; index < bound; index++) total += values[index]; return total + outer; }; } makeReduce()([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10);"
            ),
            Ok(Value::Number(95.0))
        );
        assert_eq!(dense::test_read_only_path_hits(), 1);
        assert_eq!(dense::test_read_only_iterations(), 9);
        assert_eq!(dense::test_read_only_bailouts(), 0);
    }

    #[test]
    fn read_only_dense_region_replays_only_the_failed_iteration() {
        let source = "function reduce(values, bound) { var total = 0; for (var index = 0; index < bound; index++) { total += values[index]; } return index + ':' + total; }";

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "var coercions = 0; var marker = {{ valueOf: function () {{ coercions++; return 20; }} }}; {source} reduce([1, marker, 3, 4], 4) + ':' + coercions;"
            )),
            Ok(Value::String("4:28:1".to_owned().into()))
        );
        assert_eq!(dense::test_iterations(), 2);
        assert!(dense::test_read_only_path_hits() > 0);
        assert!(dense::test_read_only_bailouts() > 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "var coercions = 0; var marker = {{ valueOf: function () {{ coercions++; return 20; }} }}; {source} reduce([1, 2, marker, 4], 4) + ':' + coercions;"
            )),
            Ok(Value::String("4:27:1".to_owned().into()))
        );
        assert!(dense::test_iterations() >= 2);
        assert!(dense::test_read_only_path_hits() > 0);
        assert!(dense::test_read_only_bailouts() > 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!("{source} reduce([1, 2], 3);")),
            Ok(Value::String("3:NaN".to_owned().into()))
        );
        assert_eq!(dense::test_iterations(), 1);
        assert!(dense::test_read_only_path_hits() > 0);
        assert!(dense::test_read_only_bailouts() > 0);
    }

    #[test]
    fn read_only_dense_region_rejects_holes_before_observable_reads() {
        let source = "function reduce(values, bound) { var total = 0; for (var index = 0; index < bound; index++) { total += values[index]; } return index + ':' + total; }";
        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "var hits = 0; var proto = {{}}; Object.defineProperty(proto, '1', {{ get: function () {{ hits++; return 5; }} }}); var values = [1, , 3]; Object.setPrototypeOf(values, proto); {source} reduce(values, 3) + ':' + hits;"
            )),
            Ok(Value::String("3:9:1".to_owned().into()))
        );
        assert_eq!(dense::test_iterations(), 0);
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert!(dense::test_read_only_bailouts() > 0);
    }

    #[test]
    fn read_only_dense_region_accepts_frozen_arrays() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function reduce(values, bound) { var total = 0; for (var index = 0; index < bound; index++) total += values[index]; return total; } var values = [1, 2, 3, 4]; Object.freeze(values); reduce(values, 4);"
            ),
            Ok(Value::Number(10.0))
        );
        assert!(dense::test_read_only_path_hits() > 0);
        assert_eq!(dense::test_read_only_bailouts(), 0);
    }

    #[test]
    fn read_only_dense_region_fails_closed_for_eval_and_captured_locals() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function direct(values, bound) { var total = 0; eval(''); for (var index = 0; index < bound; index++) total += values[index]; return index + ':' + total; } direct([1, 2, 3], 3);"
            ),
            Ok(Value::String("3:6".to_owned().into()))
        );
        assert_eq!(dense::test_iterations(), 0);
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_bailouts(), 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function captured(values, bound) { var total = 0; function read() { return index + bound; } for (var index = 0; index < bound; index++) total += values[index]; return read() + ':' + total; } captured([1, 2, 3], 3);"
            ),
            Ok(Value::String("6:6".to_owned().into()))
        );
        assert_eq!(dense::test_iterations(), 0);
        assert_eq!(dense::test_read_only_path_hits(), 0);
        assert_eq!(dense::test_read_only_bailouts(), 0);
    }

    #[test]
    fn dynamic_dense_region_handles_renamed_multi_array_loads_and_stores() {
        let source = "function project(signal, order, positive, negative, bound) { var cursor = 0; for (; cursor < bound; cursor = cursor + 3) { positive[cursor] = signal[order[cursor]] * 7 + 2; negative[cursor] = signal[order[cursor]] - 5; } return cursor + ':' + positive.join(',') + ':' + negative.join(','); }";
        let bytecode = nested_function(source);
        assert_eq!(
            NumericMutationLoopPlan::compile_all(&bytecode).len(),
            1,
            "{:#?}",
            bytecode.code
        );

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} project([4, 5, 6, 7], [2, 0, 0, 1, 0, 0, 3], [0, 0, 0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 0, 0, 0], 7);"
            )),
            Ok(Value::String(
                "9:44,0,0,37,0,0,51,0:1,0,0,0,0,0,2,0".to_owned().into()
            ))
        );
        assert!(dense::test_iterations() >= 2);
    }

    #[test]
    fn dynamic_dense_region_forwards_ordered_same_array_stores() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function bump(values, bound) { for (var position = 0; position < bound; position++) { values[position] = values[position] + 1; values[position] = values[position] + 2; } return values.join(':'); } bump([1, 2, 3, 4], 4);"
            ),
            Ok(Value::String("4:5:6:7".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn dynamic_dense_region_rejects_runtime_receiver_aliases() {
        let source = "function shift(destination, source, bound) { for (var offset = 0; offset < bound; offset++) { destination[offset + 1] = source[offset] + 1; } return destination.join(':'); }";
        let bytecode = nested_function(source);
        assert_eq!(
            NumericMutationLoopPlan::compile_all(&bytecode).len(),
            1,
            "{:#?}",
            bytecode.code
        );

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var shared = [1, 0, 0, 0]; shift(shared, shared, 3);"
            )),
            Ok(Value::String("1:2:3:4".to_owned().into()))
        );
        assert_eq!(dense::test_iterations(), 0);
    }

    #[test]
    fn dynamic_dense_region_replays_entry_and_mid_iteration_deopts() {
        let source = "var coercions = 0; var marker = { valueOf: function () { coercions++; return 20; } }; function region(left, inputs, output, bound) { for (var index = 0; index < bound; index++) { left[index] = left[index] + 1; output[index] = inputs[index] + left[index]; } return left.join(':') + '|' + output.join(':') + '|' + coercions; }";

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} region([10, 10, 10], [1, marker, 3], [0, 0, 0], 3);"
            )),
            Ok(Value::String("11:11:11|12:31:14|1".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} region([10, 10, 10, 10], [1, 2, marker, 4], [0, 0, 0, 0], 4);"
            )),
            Ok(Value::String("11:11:11:11|12:13:31:15|1".to_owned().into()))
        );
        assert!(dense::test_iterations() >= 2);
    }

    #[test]
    fn dynamic_dense_region_preflights_every_store_before_commit() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function region(left, right, bound) { for (var index = 0; index < bound; index++) { left[index] = left[index] + 1; right[index] = left[index] + 2; } return left.join(':') + '|' + right.join(':'); } region([1, 1, 1], [0, 0], 3);"
            ),
            Ok(Value::String("2:2:2|4:4:4".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn dynamic_dense_single_path_discards_staged_store_before_later_guard_failure() {
        let source = "function region(values, bound) { var observed = 0; for (var index = 0; index < bound; index++) { values[index] = values[index] + 1; observed = values[index] + values[index + 1]; } return values.join(':') + '|' + observed; }";
        let bytecode = nested_function(source);
        assert_eq!(
            NumericMutationLoopPlan::compile_all(&bytecode).len(),
            1,
            "{:#?}",
            bytecode.code
        );

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!("{source} region([1, 2, 3], 3);")),
            Ok(Value::String("2:3:4|NaN".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);
        assert!(dense::test_single_path_hits() > 0);
        assert_eq!(dense::test_sunk_store_hits(), 0);
    }

    #[test]
    fn dynamic_dense_sinks_a_unique_store_without_changing_assignment_value() {
        let source = "function shift(values, bound) { var last = 0; for (var index = 0; index < bound; index++) { last = (values[index] = values[index] + 3); } return values.join(':') + '|' + last; }";
        let bytecode = nested_function(source);
        assert_eq!(
            NumericMutationLoopPlan::compile_all(&bytecode).len(),
            1,
            "{:#?}",
            bytecode.code
        );

        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!("{source} shift([1, 2, 3, 4], 4);")),
            Ok(Value::String("4:5:6:7|7".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);
        assert!(dense::test_single_path_hits() > 0);
        assert!(dense::test_sunk_store_hits() > 0);
    }

    #[test]
    fn dynamic_dense_rmw_commits_user_local_writes_at_exit() {
        let bytecode = nested_function(
            "function run(n) { var a = [255, 255, 255, 255]; var last = -1; for (var j = 0; j < n; j = j + 1) { last = j; a[j] &= ~(1 << (j & 31)); } return last; }",
        );
        assert_eq!(
            NumericMutationLoopPlan::compile_all(&bytecode).len(),
            1,
            "{:#?}",
            bytecode.code
        );
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function run(n) { var a = [255, 255, 255, 255]; var last = -1; for (var j = 0; j < n; j = j + 1) { last = j; a[j] &= ~(1 << (j & 31)); } return last; } run(4);"
            ),
            Ok(Value::Number(3.0))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn dynamic_dense_rmw_deoptimizes_before_observable_coercion() {
        assert_eq!(
            eval(
                "function run() { var hits = 0; var bad = { valueOf: function () { hits += 1; return 7; } }; var a = [255, 255, bad, 255]; for (var j = 0; j < 4; j = j + 1) { a[j] &= ~(1 << (j & 31)); } return a.join(':') + ':' + hits; } run();"
            ),
            Ok(Value::String("254:253:3:247:1".to_owned().into()))
        );
    }

    #[test]
    fn dynamic_dense_rmw_deopt_replays_current_user_local_write_once() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "var hits = 0; var bad = { valueOf: function () { hits += 1; return 7; } }; function run(n) { var a = [255, 255, bad]; var last = -1; for (var j = 0; j < n; j = j + 1) { last = j; a[j] &= ~(1 << (j & 31)); } return last + ':' + hits + ':' + a.join(':'); } run(3);"
            ),
            Ok(Value::String("2:1:254:253:3".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn dynamic_dense_rmw_deoptimizes_before_out_of_range_store() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function run(n) { var a = [255, 255, 255, 255]; for (var j = 0; j < n; j = j + 2) { a[j] &= ~(1 << (j & 31)); } return a.length + ':' + a.join(':'); } run(6);"
            ),
            Ok(Value::String("5:254:255:251:255:0".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn sparse_dense_rmw_keeps_inherited_accessor_semantics() {
        assert_eq!(
            eval(
                "function run(n) { var hits = 0; var proto = {}; Object.defineProperty(proto, '1', { get: function () { hits += 1; return 255; } }); var a = [255, , 255, 255]; Object.setPrototypeOf(a, proto); for (var j = 0; j < n; j = j + 1) { a[j] &= ~(1 << (j & 31)); } return hits + ':' + Object.hasOwn(a, '1'); } run(4);"
            ),
            Ok(Value::String("1:false".to_owned().into()))
        );
    }

    #[test]
    fn dynamic_dense_rmw_numeric_key_edges_fail_closed() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function negativeZero(n) { var a = [255]; for (var j = 0; j < n; j = j + 1) { a[j * -0] &= ~(1 << (j & 31)); } return a[0]; } negativeZero(3);"
            ),
            Ok(Value::Number(248.0))
        );
        assert!(dense::test_iterations() > 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function nanKey(n) { var a = [1]; a.NaN = 7; for (var j = 0; j < n; j = j + 1) { a[0 / 0] &= ~(1 << (j & 31)); } return a.NaN; } nanKey(2);"
            ),
            Ok(Value::Number(4.0))
        );
        assert_eq!(dense::test_iterations(), 0);

        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function bigintElement(n) { var a = [255, 255, 1n]; var caught = false; try { for (var j = 0; j < n; j = j + 1) { a[j] &= ~(1 << (j & 31)); } } catch (error) { caught = error instanceof TypeError; } return caught + ':' + a[0] + ':' + a[1] + ':' + a[2]; } bigintElement(3);"
            ),
            Ok(Value::String("true:254:253:1".to_owned().into()))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn unrelated_virtual_literal_coexists_with_dense_mutation_plan() {
        let source = "function run(n) { var scratch = { x: 2, y: 3 }; var a = [0, 0, 0, 0]; var sum = 0; for (var i = 0; i < n; i++) { a[0] = a[3] + 1; a[1] = a[0] + 1; a[2] = a[1] - 1; a[3] = a[2]; sum += a[3]; } return sum + scratch.x + scratch.y; }";
        let bytecode = nested_function(source);
        let lowered = super::super::virtual_object::lower(&bytecode);
        let lowered_code = lowered.code(&bytecode.code);
        assert!(lowered_code.iter().any(|op| matches!(
            op,
            Op::InitVirtualObject { .. } | Op::InitVirtualConstants { .. }
        )));
        assert!(
            lowered_code
                .iter()
                .any(|op| matches!(op, Op::NewArray { .. }))
        );

        dense::reset_test_iterations();
        assert_eq!(eval(&format!("{source} run(4);")), Ok(Value::Number(15.0)));
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn dynamic_dense_rmw_ignores_prototype_for_present_own_elements() {
        assert_eq!(
            eval(
                "function run() { var hits = 0; var proto = {}; Object.defineProperty(proto, '1', { get: function () { hits += 1; return 0; } }); var a = [255, 255, 255, 255]; Object.setPrototypeOf(a, proto); for (var j = 0; j < 4; j = j + 1) { a[j] &= ~(1 << (j & 31)); } return a[0] + ':' + a[1] + ':' + a[2] + ':' + a[3] + ':' + hits; } run();"
            ),
            Ok(Value::String("254:253:251:247:0".to_owned().into()))
        );
    }

    #[test]
    fn dynamic_dense_rmw_preserves_array_integrity_rules() {
        assert_eq!(
            eval(
                "function mutate(a) { for (var j = 0; j < 4; j = j + 1) { a[j] &= ~(1 << (j & 31)); } return a[0] + ':' + a[1] + ':' + a[2] + ':' + a[3]; } var sealed = [255, 255, 255, 255]; Object.seal(sealed); Object.defineProperty(sealed, 'length', { writable: false }); var frozen = [255, 255, 255, 255]; Object.freeze(frozen); mutate(sealed) + '|' + mutate(frozen);"
            ),
            Ok(Value::String(
                "254:253:251:247|255:255:255:255".to_owned().into()
            ))
        );
    }

    #[test]
    fn runs_nested_nsieve_style_dense_rmw() {
        dense::reset_test_iterations();
        assert_eq!(
            eval(
                "function primes(n) { var bits = [-1, -1]; var count = 0; for (var i = 2; i < n; i = i + 1) { if (bits[i >> 5] & (1 << (i & 31))) { count = count + 1; for (var j = i + i; j < n; j = j + i) { bits[j >> 5] &= ~(1 << (j & 31)); } } } return count; } primes(64);"
            ),
            Ok(Value::Number(18.0))
        );
        assert!(dense::test_iterations() > 0);
    }

    #[test]
    fn commits_scalar_replaced_fields_at_loop_exit() {
        assert_eq!(
            eval(
                "function run(n) { var o = { a: 0, b: 0, c: 0 }; var sum = 0; for (var i = 0; i < n; i++) { o.a = o.c + 1; o.b = o.a + 1; o.c = o.b - 1; sum += o.c; } return sum + ':' + o.a + ':' + o.b + ':' + o.c; } run(1000);"
            ),
            Ok(Value::String("500500:1000:1001:1000".to_owned().into()))
        );
    }

    #[test]
    fn accessors_keep_the_observable_loop_path() {
        assert_eq!(
            eval(
                "function run(n) { var value = 0, writes = 0; var o = { b: 0, c: 0 }; Object.defineProperty(o, 'a', { get: function () { return value; }, set: function (next) { value = next; writes += 1; } }); var sum = 0; for (var i = 0; i < n; i++) { o.a = o.c + 1; o.b = o.a + 1; o.c = o.b - 1; sum += o.c; } return sum + ':' + writes; } run(4);"
            ),
            Ok(Value::String("10:4".to_owned().into()))
        );
    }

    #[test]
    fn read_only_fields_keep_sloppy_assignment_semantics() {
        assert_eq!(
            eval(
                "function run(n) { var o = { a: 0, b: 0, c: 0 }; Object.defineProperty(o, 'b', { writable: false }); var sum = 0; for (var i = 0; i < n; i++) { o.a = o.c + 1; o.b = o.a + 1; o.c = o.b - 1; sum += o.c; } return sum + ':' + o.a + ':' + o.b + ':' + o.c; } run(4);"
            ),
            Ok(Value::String("-4:0:0:-1".to_owned().into()))
        );
    }
}
