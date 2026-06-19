use crate::statement::Stmt;

/// A JavaScript script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Script {
    /// Top-level statements.
    pub body: Vec<Stmt>,
    /// The original source text, retained so the runtime can reproduce a
    /// function's source for `Function.prototype.toString` (sliced by each
    /// function's span). Empty for synthesized scripts.
    pub source: std::rc::Rc<str>,
}
