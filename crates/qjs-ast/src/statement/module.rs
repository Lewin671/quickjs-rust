use crate::expression::Expr;
use crate::span::Span;
use crate::statement::Stmt;

/// A module-level item: an `import` or `export` declaration. These appear only
/// in source parsed under the Module goal symbol; the runtime currently rejects
/// them with a structured "modules are not yet supported" error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleDecl {
    /// An `import` declaration.
    Import(ImportDecl),
    /// An `export` declaration.
    Export(ExportDecl),
}

impl ModuleDecl {
    /// Source span covering the whole declaration.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Import(decl) => decl.span,
            Self::Export(decl) => decl.span(),
        }
    }
}

/// An `import` declaration.
///
/// Covers the four forms that share the `ImportDeclaration` production:
/// default (`import x from "m"`), named (`import {a as b} from "m"`),
/// namespace (`import * as ns from "m"`), and side-effect
/// (`import "m"`). The forms may combine (`import x, {a} from "m"`,
/// `import x, * as ns from "m"`). Import assertions are not represented.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportDecl {
    /// The clauses binding names from the module, empty for a side-effect
    /// import.
    pub specifiers: Vec<ImportSpecifier>,
    /// The module specifier string (the `"m"` in `from "m"`).
    pub source: String,
    /// Source span.
    pub span: Span,
}

/// A single binding introduced by an `import` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportSpecifier {
    /// `import x from "m"` — binds the module's default export to `local`.
    Default {
        /// Local binding name.
        local: String,
        /// Source span.
        span: Span,
    },
    /// `import * as ns from "m"` — binds the module namespace object.
    Namespace {
        /// Local binding name.
        local: String,
        /// Source span.
        span: Span,
    },
    /// `import {a as b} from "m"` — binds the named export `imported` to the
    /// local name `local`. When the source uses the shorthand `import {a}`,
    /// `imported` and `local` are equal.
    Named {
        /// Exported name in the source module.
        imported: ModuleExportName,
        /// Local binding name.
        local: String,
        /// Source span.
        span: Span,
    },
}

/// An `export` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExportDecl {
    /// `export {a, b as c}` or `export {a, b as c} from "m"`. When `source` is
    /// present this is a re-export and the names refer to the source module.
    Named {
        /// Exported bindings.
        specifiers: Vec<ExportSpecifier>,
        /// Re-export source module specifier, if any.
        source: Option<String>,
        /// Source span.
        span: Span,
    },
    /// `export * from "m"` (no `exported`) or `export * as ns from "m"`.
    All {
        /// The local export name for `export * as ns from "m"`, or `None` for
        /// the bare `export *` star re-export.
        exported: Option<ModuleExportName>,
        /// Source module specifier.
        source: String,
        /// Source span.
        span: Span,
    },
    /// `export default <expression>` / `export default function ...` /
    /// `export default class ...`.
    Default {
        /// The exported default value.
        declaration: DefaultExport,
        /// Source span.
        span: Span,
    },
    /// `export var/let/const ...`, `export function ...`, or `export class ...`:
    /// a declaration whose bound names become exports.
    Declaration {
        /// The wrapped declaration statement.
        declaration: Box<Stmt>,
        /// Source span.
        span: Span,
    },
}

impl ExportDecl {
    /// Source span covering the whole declaration.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Named { span, .. }
            | Self::All { span, .. }
            | Self::Default { span, .. }
            | Self::Declaration { span, .. } => *span,
        }
    }
}

/// The payload of an `export default` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefaultExport {
    /// `export default function ...` or `export default class ...`. The name is
    /// optional in these positions.
    Declaration(Box<Stmt>),
    /// `export default <AssignmentExpression>`.
    Expression(Expr),
}

/// A single binding listed in an `export {...}` clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportSpecifier {
    /// The local name (or, for a re-export, the imported name) being exported.
    pub local: ModuleExportName,
    /// The name under which it is exported. Equal to `local` for the shorthand
    /// `export {a}`.
    pub exported: ModuleExportName,
    /// Source span.
    pub span: Span,
}

/// A name in an import/export clause. An `IdentifierName` or, since ES2022, a
/// `StringLiteral` (for names that are not valid identifiers, used with
/// re-exports and namespace exports).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleExportName {
    /// An identifier name such as `foo`.
    Identifier(String),
    /// A string-literal name such as `"a-b"`.
    String(String),
}

impl ModuleExportName {
    /// The underlying string value of the name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Identifier(name) | Self::String(name) => name,
        }
    }
}
