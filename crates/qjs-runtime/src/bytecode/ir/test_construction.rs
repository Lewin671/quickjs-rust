//! Raw IR constructors for tests without a source compiler.

use super::{Bytecode, Local, Op};
use crate::Value;
use qjs_ast::FunctionParams;

impl Bytecode {
    pub(in crate::bytecode) fn new(
        constants: Vec<Value>,
        locals: Vec<Local>,
        code: Vec<Op>,
    ) -> Self {
        Self::with_scope(constants, locals, code, false)
    }

    pub(in crate::bytecode) fn new_function(
        constants: Vec<Value>,
        locals: Vec<Local>,
        code: Vec<Op>,
        parameter_slots: Vec<usize>,
        params: &FunctionParams,
    ) -> Self {
        let mut bytecode = Self::with_scope_global_lexical_names_and_strict_with_parameter_slots(
            constants,
            locals,
            code,
            false,
            Vec::new(),
            false,
            Some(parameter_slots),
            None,
        );
        bytecode.binding_plan.parameters =
            super::super::binding_plan::ParameterInfo::compile(&bytecode, params);
        bytecode
    }

    pub(in crate::bytecode) fn with_scope(
        constants: Vec<Value>,
        locals: Vec<Local>,
        code: Vec<Op>,
        global_scope: bool,
    ) -> Self {
        Self::with_scope_and_global_lexical_names(constants, locals, code, global_scope, Vec::new())
    }

    pub(in crate::bytecode) fn with_scope_and_global_lexical_names(
        constants: Vec<Value>,
        locals: Vec<Local>,
        code: Vec<Op>,
        global_scope: bool,
        global_lexical_names: Vec<String>,
    ) -> Self {
        Self::with_scope_global_lexical_names_and_strict(
            constants,
            locals,
            code,
            global_scope,
            global_lexical_names,
            false,
        )
    }

    pub(in crate::bytecode) fn with_scope_global_lexical_names_and_strict(
        constants: Vec<Value>,
        locals: Vec<Local>,
        code: Vec<Op>,
        global_scope: bool,
        global_lexical_names: Vec<String>,
        strict: bool,
    ) -> Self {
        Self::with_scope_global_lexical_names_and_strict_with_parameter_slots(
            constants,
            locals,
            code,
            global_scope,
            global_lexical_names,
            strict,
            None,
            None,
        )
    }
}
