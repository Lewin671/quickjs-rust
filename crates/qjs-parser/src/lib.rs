//! Parser for a small JavaScript subset.

mod cursor;
mod expression;
mod helpers;
mod statement;

use qjs_ast::{Script, Span};
use qjs_lexer::{Token, lex};

/// A parse error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    /// Human-readable message.
    pub message: String,
    /// Source span.
    pub span: Span,
}

/// Parses source text into a script AST.
///
/// # Errors
///
/// Returns a structured error for lexing or parsing failures.
pub fn parse_script(source: &str) -> Result<Script, ParseError> {
    let tokens = lex(source).map_err(|error| ParseError {
        message: error.message,
        span: error.span,
    })?;
    Parser::new(tokens, source.to_owned()).parse_script()
}

struct Parser {
    source: String,
    tokens: Vec<Token>,
    cursor: usize,
    strict: bool,
    allow_in: bool,
    /// Whether `super.prop`/`super[expr]` member access is currently allowed,
    /// i.e. the parser is inside a method or accessor body (or an arrow nested
    /// in one).
    in_method: bool,
    /// Whether `super(...)` calls are currently allowed, i.e. the parser is
    /// inside a derived-class constructor body (or an arrow nested in one).
    in_derived_constructor: bool,
    /// Whether the parser is inside a class field initializer expression, where
    /// `arguments` is a syntax error.
    in_field_initializer: bool,
    /// Whether the parser is inside a generator function body (or an arrow
    /// nested in one), where `yield` is a keyword introducing a yield
    /// expression. Ordinary nested functions reset this; arrow functions
    /// inherit it.
    in_generator: bool,
    /// Whether the parser is inside a generator's formal parameter list, where
    /// a `yield` expression is an early syntax error.
    in_generator_params: bool,
    /// Stack of private-name scopes. Each entry holds the private names declared
    /// by one class body currently being parsed; the innermost class is last.
    /// A private reference resolves against any scope in the stack.
    private_scopes: Vec<PrivateScope>,
    /// Private-name references seen but not yet resolved to a declaring class.
    /// Each is retried as classes close; any left when the outermost class
    /// closes (or at top level) is a syntax error.
    pending_private_refs: Vec<PendingPrivateRef>,
}

/// The set of private names declared by one class body, plus accessor tracking
/// so a getter/setter pair for the same name is not flagged as a duplicate.
#[derive(Default)]
struct PrivateScope {
    /// Declared private names and the kind of declaration, for duplicate
    /// detection.
    declarations: Vec<PrivateDeclaration>,
}

impl PrivateScope {
    fn declares(&self, name: &str) -> bool {
        self.declarations
            .iter()
            .any(|declaration| declaration.name == name)
    }
}

struct PrivateDeclaration {
    name: String,
    kind: PrivateDeclKind,
    is_static: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrivateDeclKind {
    Field,
    Method,
    Getter,
    Setter,
}

struct PendingPrivateRef {
    name: String,
    span: qjs_ast::Span,
}

#[cfg(test)]
mod tests;
