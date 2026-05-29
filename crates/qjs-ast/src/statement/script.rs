use crate::statement::Stmt;

/// A JavaScript script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Script {
    /// Top-level statements.
    pub body: Vec<Stmt>,
}
