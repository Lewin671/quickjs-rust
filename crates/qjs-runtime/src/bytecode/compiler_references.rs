//! Record reference evaluation before compiling RHS effects; no extra executable
//! instruction is inserted and every site retains its original lexical scope.
use super::{
    compiler::Compiler,
    reference_plan::{ReferenceDraft, ReferenceId},
};

impl Compiler {
    pub(super) fn begin_reference(&mut self, name: &str) -> ReferenceId {
        let id = ReferenceId(self.references.len());
        self.references.push(ReferenceDraft {
            name: name.to_owned(),
            start: self.code.len(),
            scope: *self
                .scope_plan
                .active
                .last()
                .expect("active compiler scope"),
            strict: self.strict,
            read: None,
            store: None,
            end: self.code.len(),
        });
        id
    }
    pub(super) fn finish_reference(&mut self, id: ReferenceId, read: Option<usize>, store: usize) {
        let site = &mut self.references[id.0];
        site.read = read;
        site.store = Some(store);
        site.end = self.code.len();
    }
}
