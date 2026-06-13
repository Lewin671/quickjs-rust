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

/// Additional parser context for direct eval code.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvalParseContext {
    /// Whether eval runs inside a function-like context, allowing
    /// `new.target`.
    pub in_function: bool,
    /// Whether eval runs inside a method-like context, allowing `super.x`.
    pub in_method: bool,
    /// Whether eval runs inside a derived constructor, allowing `super()`.
    pub in_derived_constructor: bool,
    /// Whether eval runs inside a class field initializer, where `arguments`
    /// is an early error.
    pub in_field_initializer: bool,
    /// Private names visible through the caller's active private environment,
    /// without the leading `#`.
    pub private_names: Vec<String>,
}

/// Parses direct-eval source text using the syntactic context of the eval call.
///
/// Indirect eval should continue to use [`parse_script`], because it parses as
/// global script code with no caller lexical context.
pub fn parse_direct_eval_script(
    source: &str,
    context: EvalParseContext,
) -> Result<Script, ParseError> {
    let tokens = lex(source).map_err(|error| ParseError {
        message: error.message,
        span: error.span,
    })?;
    let mut parser = Parser::new(tokens, source.to_owned());
    parser.in_function = context.in_function;
    parser.in_method = context.in_method;
    parser.in_derived_constructor = context.in_derived_constructor;
    parser.in_field_initializer = context.in_field_initializer;
    if !context.private_names.is_empty() {
        parser.private_scopes.push(PrivateScope {
            declarations: context
                .private_names
                .into_iter()
                .map(|name| PrivateDeclaration {
                    name,
                    kind: PrivateDeclKind::Field,
                    is_static: false,
                })
                .collect(),
        });
    }
    parser.parse_script()
}

/// Parses source text as a module (the Module goal symbol).
///
/// Module source permits top-level `import` and `export` declarations and is
/// always strict mode. The returned [`Script`] body contains the module items
/// as [`qjs_ast::Stmt::ModuleDecl`] entries alongside ordinary statements.
///
/// # Errors
///
/// Returns a structured error for lexing or parsing failures.
pub fn parse_module(source: &str) -> Result<Script, ParseError> {
    let tokens = lex(source).map_err(|error| ParseError {
        message: error.message,
        span: error.span,
    })?;
    let mut parser = Parser::new(tokens, source.to_owned());
    parser.goal = Goal::Module;
    parser.strict = true;
    // Under the Module goal the top level is an `[+Await]` context: `await expr`
    // is an AwaitExpression and `await` may not be used as an identifier or
    // binding. Ordinary (non-async) nested functions reset this context, so
    // `await` is an identifier again inside them. Reuse the async-await context
    // flag the rest of the parser already keys off.
    parser.in_async = true;
    parser.parse_script()
}

/// The grammar goal symbol the parser is operating under. Module source allows
/// top-level `import`/`export` and is implicitly strict; script source does
/// not and treats `import`/`export` identifiers as ordinary names.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Goal {
    Script,
    Module,
}

struct Parser {
    source: String,
    tokens: Vec<Token>,
    cursor: usize,
    /// The grammar goal symbol: script or module. Module source allows
    /// top-level `import`/`export` declarations.
    goal: Goal,
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
    /// Whether `new.target` is currently allowed. Function, method, static
    /// block, and field-initializer bodies are function-like contexts for this
    /// early error; arrow functions inherit the enclosing setting.
    in_function: bool,
    /// Whether the parser is inside a class static block statement list. Static
    /// blocks have dedicated early errors for `return`, `await`, `yield`, and
    /// `arguments`; ordinary nested functions and methods reset this context.
    in_static_block: bool,
    /// Whether the parser is inside a generator function body (or an arrow
    /// nested in one), where `yield` is a keyword introducing a yield
    /// expression. Ordinary nested functions reset this; arrow functions
    /// inherit it.
    in_generator: bool,
    /// Whether the parser is inside a generator's formal parameter list, where
    /// a `yield` expression is an early syntax error.
    in_generator_params: bool,
    /// Whether the parser is inside an async function body (or an arrow nested
    /// in one), where `await` is a keyword introducing an await expression and
    /// `await` may not be used as an identifier. Ordinary nested functions
    /// reset this; arrow functions inherit it.
    in_async: bool,
    /// Whether the parser is inside an async function's formal parameter list,
    /// where an `await` expression is an early syntax error.
    in_async_params: bool,
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
