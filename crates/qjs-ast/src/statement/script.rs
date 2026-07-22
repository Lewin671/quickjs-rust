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
    /// Whether `source` already uses the runtime's canonical WTF-16 sentinel
    /// representation. Host UTF-8 source is retained verbatim for byte spans
    /// and canonicalized only when a slice becomes a JavaScript String.
    pub source_is_wtf16: bool,
}
