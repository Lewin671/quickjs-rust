//! Real guard failure after identity selection preserves the physical continuation.

use std::{collections::HashMap, rc::Rc};

use super::{LoopFrame, compile_all};
use crate::bytecode::{
    ir::{Bytecode, Op},
    vm_bindings::TypedLoopSloppyGlobalWrite,
    vm_loop_dispatch::LoopPlanView,
};
use crate::{ArrayRef, CallEnv, ObjectRef, Property, RuntimeError, Value, eval};

struct Frame<'a> {
    code: &'a Bytecode,
    env: CallEnv,
    locals: Vec<Option<Value>>,
    stack: Vec<Value>,
    resume: Option<(usize, bool)>,
    declined: u128,
}

impl LoopFrame for Frame<'_> {
    fn loop_env(&self) -> &CallEnv {
        &self.env
    }
    fn direct_eval_with_stack(&self) -> bool {
        false
    }
    fn declined_typed_loop_programs(&self) -> u128 {
        self.declined
    }
    fn decline_typed_loop_program(&mut self, bit: u128) {
        self.declined |= bit;
    }
    fn local_slot_value(&self, slot: usize) -> Option<Value> {
        self.locals.get(slot)?.clone()
    }
    fn slot_accepts_typed_loop_write(&self, _: usize) -> bool {
        true
    }
    fn write_typed_loop_slot(&mut self, slot: usize, value: Value) {
        assert!(slot < self.code.locals.len());
        self.locals[slot] = Some(value);
    }
    fn load_global(&mut self, _: &str) -> Result<Value, RuntimeError> {
        panic!("this region has no global reads")
    }
    fn global_this_own_property(&self, _: &str) -> Option<Property> {
        None
    }
    fn array_access_is_plain(&mut self, _: &ArrayRef) -> bool {
        false
    }
    fn try_create_ordinary_own_data_property(&self, _: &ObjectRef, _: Rc<str>, _: &Value) -> bool {
        false
    }
    fn prepare_typed_loop_sloppy_global_write(
        &self,
        _: usize,
        _: &str,
    ) -> Option<TypedLoopSloppyGlobalWrite> {
        None
    }
    fn write_typed_loop_sloppy_global(&mut self, _: &TypedLoopSloppyGlobalWrite, _: Value) -> bool {
        false
    }
    fn record_sloppy_global_name(&mut self, _: &str) {
        panic!("no global sink")
    }
    fn push_stack(&mut self, value: Value) {
        self.stack.push(value);
    }
    fn resume_at(&mut self, ip: usize, deoptimized: bool) {
        self.resume = Some((ip, deoptimized));
    }
    #[cfg(feature = "perf-counters")]
    fn bytecode_op(&self, ip: usize) -> Option<&Op> {
        self.code.code.get(ip)
    }
}

#[test]
fn identity_receiver_guard_restores_method_prefix_and_exact_get_site() {
    let source = "function run(a,b,n){var s=0,r,g=0;for(var i=0;i<n&&g<9;i++,g++){r=i<2?a:b;s+=r.verb(i);}return s;}";
    let script = qjs_parser::parse_script(source).unwrap();
    let script = super::super::compile_script(&script).unwrap();
    let code = script
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref()),
            _ => None,
        })
        .unwrap();
    let plans = compile_all(code);
    let plan = plans.first().unwrap();
    let get_ip = code
        .code
        .iter()
        .position(|op| matches!(op, Op::GetPropNamed { key, .. } if key.as_ref()=="verb"))
        .unwrap();
    let slot = |name: &str| {
        code.locals
            .iter()
            .position(|local| local.name == name)
            .unwrap()
    };
    let good = eval("({verb:function(n){return n+10;}});").unwrap();
    for bad in [
        eval("({get verb(){throw Error('getter must wait for continuation');}});").unwrap(),
        eval("new Proxy({}, {get(){throw Error('proxy must wait for continuation');}});").unwrap(),
        Value::Null,
    ] {
        let mut frame = Frame {
            code,
            env: CallEnv::new(crate::function::new_realm(HashMap::new())),
            locals: vec![Some(Value::Undefined); code.locals.len()],
            stack: Vec::new(),
            resume: None,
            declined: 0,
        };
        for (name, value) in [
            ("a", good.clone()),
            ("b", bad.clone()),
            ("n", Value::Number(6.0)),
            ("i", Value::Number(1.0)),
            ("g", Value::Number(1.0)),
            ("s", Value::Number(10.0)),
            ("r", good.clone()),
        ] {
            frame.locals[slot(name)] = Some(value);
        }
        assert!(super::try_run_typed_loop(
            &mut frame,
            LoopPlanView {
                typed: &plans,
                shared_numeric_mutation: &[]
            },
            plan.header(),
            plan.backedge()
        ));
        assert_eq!(frame.resume, Some((get_ip, true)));
        assert_eq!(frame.stack, [Value::Number(21.0), bad.clone(), bad.clone()]);
        assert_eq!(frame.locals[slot("s")], Some(Value::Number(21.0)));
        assert_eq!(frame.locals[slot("i")], Some(Value::Number(2.0)));
        assert_eq!(frame.locals[slot("r")], Some(bad));
    }
}
