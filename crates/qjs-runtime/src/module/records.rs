//! Source Text Module Record construction.
//!
//! Parses a module's source under the Module goal and collects its import and
//! export entries, mirroring the static-semantics tables of ECMAScript 16.2.1.3
//! (`ImportEntries`, `ExportEntries` split into local / indirect / star). The
//! module body is also rewritten into a plain [`Script`] whose `import`/`export`
//! wrappers are stripped, so the existing global-scope compiler can lower it.

use qjs_ast::{DefaultExport, ExportDecl, ImportSpecifier, ModuleDecl, Script, Stmt, VarKind};

/// The synthetic local-binding name holding a module's default export. It is
/// not a valid identifier, so it never collides with a user binding.
pub(super) const DEFAULT_BINDING: &str = "*default*";
pub(super) const NAMESPACE_BINDING: &str = "*namespace*";

/// An `import` entry: a local binding fed from another module.
#[derive(Clone, Debug)]
pub(super) struct ImportEntry {
    /// The module specifier the binding is imported from.
    pub(super) module_request: String,
    /// The name imported from that module, or [`ImportName::Namespace`] for a
    /// namespace import (`import * as ns`).
    pub(super) import_name: ImportName,
    /// The local binding name introduced in this module.
    pub(super) local_name: String,
}

/// What an import binds: a single named export, the default export, or the whole
/// namespace object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ImportName {
    /// A named (or default) export, by its export name.
    Named(String),
    /// `import * as ns` — the module namespace object.
    Namespace,
}

/// A local export entry (`export {x}`, `export const x`, `export default ...`).
#[derive(Clone, Debug)]
pub(super) struct LocalExportEntry {
    /// The name the binding is exported under.
    pub(super) export_name: String,
    /// The local binding name within this module.
    pub(super) local_name: String,
}

/// An indirect (re-)export entry (`export {x} from "m"`,
/// `export * as ns from "m"`).
#[derive(Clone, Debug)]
pub(super) struct IndirectExportEntry {
    /// The name this module exports the binding under.
    pub(super) export_name: String,
    /// The module specifier the binding comes from.
    pub(super) module_request: String,
    /// The name within the target module, or [`ImportName::Namespace`] for
    /// `export * as ns from`.
    pub(super) import_name: ImportName,
}

/// The collected static entries of a Source Text Module Record.
pub(super) struct ModuleRecord {
    /// Distinct module specifiers this module imports/re-exports from.
    pub(super) requested_modules: Vec<String>,
    pub(super) import_entries: Vec<ImportEntry>,
    pub(super) local_exports: Vec<LocalExportEntry>,
    pub(super) indirect_exports: Vec<IndirectExportEntry>,
    /// Specifiers of bare `export * from "m"` star re-exports.
    pub(super) star_exports: Vec<String>,
    /// The module body with import/export wrappers stripped, ready to compile.
    pub(super) body: Script,
}

/// Parses `source` under the Module goal and collects its module record.
pub(super) fn build_record(source: &str) -> Result<ModuleRecord, String> {
    let script = qjs_parser::parse_module(source).map_err(|error| error.message)?;
    let mut record = ModuleRecord {
        requested_modules: Vec::new(),
        import_entries: Vec::new(),
        local_exports: Vec::new(),
        indirect_exports: Vec::new(),
        star_exports: Vec::new(),
        body: Script {
            body: Vec::new(),
            source: script.source.clone(),
        },
    };
    for stmt in script.body {
        match stmt {
            Stmt::ModuleDecl(ModuleDecl::Import(import)) => {
                record.request(&import.source);
                for specifier in import.specifiers {
                    record.collect_import(&import.source, specifier);
                }
            }
            Stmt::ModuleDecl(ModuleDecl::Export(export)) => {
                record.collect_export(export);
            }
            other => record.body.body.push(other),
        }
    }
    Ok(record)
}

impl ModuleRecord {
    fn request(&mut self, specifier: &str) {
        if !self.requested_modules.iter().any(|m| m == specifier) {
            self.requested_modules.push(specifier.to_owned());
        }
    }

    fn collect_import(&mut self, module_request: &str, specifier: ImportSpecifier) {
        let (local_name, import_name) = match specifier {
            ImportSpecifier::Default { local, .. } => {
                (local, ImportName::Named("default".to_owned()))
            }
            ImportSpecifier::Namespace { local, .. } => (local, ImportName::Namespace),
            ImportSpecifier::Named {
                imported, local, ..
            } => (local, ImportName::Named(imported.as_str().to_owned())),
        };
        self.import_entries.push(ImportEntry {
            module_request: module_request.to_owned(),
            import_name,
            local_name,
        });
    }

    fn collect_export(&mut self, export: ExportDecl) {
        match export {
            ExportDecl::Named {
                specifiers, source, ..
            } => {
                if let Some(source) = source {
                    self.request(&source);
                    for specifier in specifiers {
                        self.indirect_exports.push(IndirectExportEntry {
                            export_name: specifier.exported.as_str().to_owned(),
                            module_request: source.clone(),
                            import_name: ImportName::Named(specifier.local.as_str().to_owned()),
                        });
                    }
                } else {
                    for specifier in specifiers {
                        self.local_exports.push(LocalExportEntry {
                            export_name: specifier.exported.as_str().to_owned(),
                            local_name: specifier.local.as_str().to_owned(),
                        });
                    }
                }
            }
            ExportDecl::All {
                exported, source, ..
            } => {
                self.request(&source);
                match exported {
                    Some(name) => self.indirect_exports.push(IndirectExportEntry {
                        export_name: name.as_str().to_owned(),
                        module_request: source,
                        import_name: ImportName::Namespace,
                    }),
                    None => self.star_exports.push(source),
                }
            }
            ExportDecl::Default { declaration, span } => {
                self.local_exports.push(LocalExportEntry {
                    export_name: "default".to_owned(),
                    local_name: DEFAULT_BINDING.to_owned(),
                });
                self.body.body.push(default_export_stmt(declaration, span));
            }
            ExportDecl::Declaration { declaration, .. } => {
                for name in declared_names(&declaration) {
                    self.local_exports.push(LocalExportEntry {
                        export_name: name.clone(),
                        local_name: name,
                    });
                }
                self.body.body.push(*declaration);
            }
        }
    }
}

/// Lowers `export default <decl|expr>` to a `const *default* = <value>;`
/// binding so the body compiler creates the synthetic default binding.
fn default_export_stmt(declaration: DefaultExport, span: qjs_ast::Span) -> Stmt {
    use qjs_ast::VarDeclarator;
    let init = match declaration {
        // A named `export default function f(){}` / `class C {}` also binds its
        // own name in module scope; for S2 we bind only `*default*` to the
        // value, which covers anonymous and named default exports uniformly.
        DefaultExport::Declaration(stmt) => stmt_to_expr(*stmt, span),
        DefaultExport::Expression(expr) => default_export_expr(expr),
    };
    Stmt::VarDecl {
        kind: VarKind::Const,
        declarations: vec![VarDeclarator {
            binding: qjs_ast::BindingPattern::Identifier {
                name: DEFAULT_BINDING.to_owned(),
                span,
            },
            init: Some(init),
            span,
        }],
        span,
    }
}

/// Applies default-export name inference to anonymous function/class
/// expressions before lowering through the synthetic local binding.
fn default_export_expr(expr: qjs_ast::Expr) -> qjs_ast::Expr {
    use qjs_ast::Expr;
    match expr {
        Expr::Function {
            name: None,
            params,
            body,
            constructable,
            lexical_this,
            lexical_arguments,
            is_generator,
            is_async,
            span,
        } => Expr::Function {
            name: Some("default".to_owned()),
            params,
            body,
            constructable,
            lexical_this,
            lexical_arguments,
            is_generator,
            is_async,
            span,
        },
        Expr::Class {
            name: None,
            body,
            span,
        } => Expr::Class {
            name: Some("default".to_owned()),
            body,
            span,
        },
        other => other,
    }
}

/// Converts a `function`/`class` declaration used as `export default` into the
/// equivalent expression value.
fn stmt_to_expr(stmt: Stmt, span: qjs_ast::Span) -> qjs_ast::Expr {
    use qjs_ast::Expr;
    match stmt {
        Stmt::FunctionDecl {
            name,
            params,
            body,
            is_async,
            is_generator,
            span,
        } => Expr::Function {
            name: Some(name),
            params,
            body,
            constructable: !is_async && !is_generator,
            lexical_this: false,
            lexical_arguments: false,
            is_generator,
            is_async,
            span,
        },
        Stmt::ClassDecl { name, body, span } => Expr::Class {
            name: Some(name),
            body,
            span,
        },
        // A non-declaration default (already handled as an expression) cannot
        // reach here; fall back to `undefined` defensively.
        _ => Expr::Identifier {
            name: "undefined".to_owned(),
            span,
        },
    }
}

/// The bound names of an exported declaration (`export var/let/const/function/
/// class`).
fn declared_names(stmt: &Stmt) -> Vec<String> {
    match stmt {
        Stmt::VarDecl { declarations, .. } => declarations
            .iter()
            .flat_map(|declaration| declaration.binding.names())
            .collect(),
        Stmt::FunctionDecl { name, .. } | Stmt::ClassDecl { name, .. } => vec![name.clone()],
        _ => Vec::new(),
    }
}
