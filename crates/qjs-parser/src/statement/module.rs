use qjs_ast::{
    DefaultExport, ExportDecl, ExportSpecifier, ImportDecl, ImportSpecifier, ModuleDecl,
    ModuleExportName, Span, Stmt,
};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    /// Parses one top-level item of a module: an `import`/`export` declaration
    /// or an ordinary statement. Only called under the Module goal symbol.
    pub(super) fn module_item(&mut self) -> Result<Stmt, ParseError> {
        if self.at_import_declaration() {
            return self
                .import_declaration()
                .map(|decl| Stmt::ModuleDecl(ModuleDecl::Import(decl)));
        }
        if self.at_export_keyword() {
            return self
                .export_declaration()
                .map(|decl| Stmt::ModuleDecl(ModuleDecl::Export(decl)));
        }
        self.statement_list_item()
    }

    /// Reports whether the cursor is at an `import` declaration (as opposed to
    /// an `import(...)` call or `import.meta` expression, which are ordinary
    /// expression statements).
    fn at_import_declaration(&self) -> bool {
        if !self.at_contextual("import") {
            return false;
        }
        !matches!(
            self.peek_nth(1).map(|token| &token.kind),
            Some(TokenKind::LeftParen | TokenKind::Dot)
        )
    }

    fn at_export_keyword(&self) -> bool {
        self.at_contextual("export")
    }

    /// Reports whether the current token is the unescaped contextual keyword
    /// `keyword`.
    fn at_contextual(&self, keyword: &str) -> bool {
        matches!(
            self.peek(),
            Some(token)
                if !token.had_escape
                    && matches!(&token.kind, TokenKind::Identifier(name) if name == keyword)
        )
    }

    fn import_declaration(&mut self) -> Result<ImportDecl, ParseError> {
        let start = self.advance().span.start; // consume `import`

        // Side-effect import: `import "mod";`
        if let Some(source) = self.try_string_literal() {
            let end = self.finish_module_specifier(source.span);
            return Ok(ImportDecl {
                specifiers: Vec::new(),
                source: source.value,
                span: Span::new(start, end),
            });
        }

        let mut specifiers = Vec::new();

        // Default binding: `import x` (optionally followed by `, {...}` or
        // `, * as ns`).
        if let Some(token) = self.peek() {
            if let TokenKind::Identifier(_) = &token.kind {
                let local = self.binding_name()?;
                specifiers.push(ImportSpecifier::Default {
                    local: local.0,
                    span: local.1,
                });
                if self.match_kind(&TokenKind::Comma) {
                    self.import_after_default(&mut specifiers)?;
                }
            } else {
                self.import_after_default(&mut specifiers)?;
            }
        }

        self.expect_contextual("from")?;
        let source = self.expect_string_literal()?;
        let end = self.finish_module_specifier(source.span);
        Ok(ImportDecl {
            specifiers,
            source: source.value,
            span: Span::new(start, end),
        })
    }

    /// Parses the namespace or named clause that may follow `import default,`.
    fn import_after_default(
        &mut self,
        specifiers: &mut Vec<ImportSpecifier>,
    ) -> Result<(), ParseError> {
        if self.at(&TokenKind::Star) {
            let (local, span) = self.namespace_clause()?;
            specifiers.push(ImportSpecifier::Namespace { local, span });
        } else if self.at(&TokenKind::LeftBrace) {
            self.named_import_clause(specifiers)?;
        } else {
            let token = self.peek().expect("parser should always have eof token");
            return Err(ParseError {
                message: "expected `*` or `{` in import clause".to_owned(),
                span: token.span,
            });
        }
        Ok(())
    }

    /// Parses `* as ns`, returning the local name and its span (including the
    /// star).
    fn namespace_clause(&mut self) -> Result<(String, Span), ParseError> {
        let start = self.advance().span.start; // consume `*`
        self.expect_contextual("as")?;
        let (local, span) = self.binding_name()?;
        Ok((local, Span::new(start, span.end)))
    }

    /// Parses `{ a, b as c }` import specifiers.
    fn named_import_clause(
        &mut self,
        specifiers: &mut Vec<ImportSpecifier>,
    ) -> Result<(), ParseError> {
        self.expect(&TokenKind::LeftBrace)?;
        while !self.at(&TokenKind::RightBrace) {
            let imported = self.module_export_name()?;
            let start = self.previous_span().start;
            let (local, local_span) = if self.match_contextual_keyword("as") {
                self.binding_name()?
            } else {
                let ModuleExportName::Identifier(name) = &imported.0 else {
                    return Err(ParseError {
                        message: "a string-named import requires `as`".to_owned(),
                        span: imported.1,
                    });
                };
                (name.clone(), imported.1)
            };
            specifiers.push(ImportSpecifier::Named {
                imported: imported.0,
                local,
                span: Span::new(start, local_span.end),
            });
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RightBrace)?;
        Ok(())
    }

    fn export_declaration(&mut self) -> Result<ExportDecl, ParseError> {
        let start = self.advance().span.start; // consume `export`

        // `export * ...` / `export * as ns from "m"`.
        if self.at(&TokenKind::Star) {
            self.advance();
            let exported = if self.match_contextual_keyword("as") {
                Some(self.module_export_name()?.0)
            } else {
                None
            };
            self.expect_contextual("from")?;
            let source = self.expect_string_literal()?;
            let end = self.finish_module_specifier(source.span);
            return Ok(ExportDecl::All {
                exported,
                source: source.value,
                span: Span::new(start, end),
            });
        }

        // `export { ... }` or `export { ... } from "m"`.
        if self.at(&TokenKind::LeftBrace) {
            let specifiers = self.export_specifier_list()?;
            let source = if self.match_contextual_keyword("from") {
                let source = self.expect_string_literal()?;
                Some(source.value)
            } else {
                None
            };
            let end = self.finish_module_specifier(self.previous_span());
            return Ok(ExportDecl::Named {
                specifiers,
                source,
                span: Span::new(start, end),
            });
        }

        // `export default ...`.
        if self.match_kind(&TokenKind::Default) {
            let declaration = self.default_export()?;
            let end = self.previous_span().end;
            return Ok(ExportDecl::Default {
                declaration,
                span: Span::new(start, end),
            });
        }

        // `export var/let/const/function/class ...`.
        let declaration = self.statement()?;
        if !is_exportable_declaration(&declaration) {
            return Err(ParseError {
                message: "`export` expects a declaration, `{...}`, `*`, or `default`".to_owned(),
                span: Span::new(start, crate::helpers::stmt_end(&declaration)),
            });
        }
        let end = crate::helpers::stmt_end(&declaration);
        Ok(ExportDecl::Declaration {
            declaration: Box::new(declaration),
            span: Span::new(start, end),
        })
    }

    fn default_export(&mut self) -> Result<DefaultExport, ParseError> {
        if self.at_async_function_keyword() {
            if self.default_function_has_name(true) {
                let stmt = self.statement()?;
                return Ok(DefaultExport::Declaration(Box::new(stmt)));
            }
            let async_token = self.advance();
            self.expect(&TokenKind::Function)?;
            let expr = self.function_expression_with_async(async_token.span.start, true)?;
            self.match_kind(&TokenKind::Semicolon);
            return Ok(DefaultExport::Expression(expr));
        }
        if self.at(&TokenKind::Function) {
            if self.default_function_has_name(false) {
                let stmt = self.statement()?;
                return Ok(DefaultExport::Declaration(Box::new(stmt)));
            }
            let start = self.advance().span.start;
            let expr = self.function_expression(start)?;
            self.match_kind(&TokenKind::Semicolon);
            return Ok(DefaultExport::Expression(expr));
        }
        if self.at(&TokenKind::Class) {
            let expr = self.assignment()?;
            self.match_kind(&TokenKind::Semicolon);
            return Ok(DefaultExport::Expression(expr));
        }
        let expr = self.assignment()?;
        self.match_kind(&TokenKind::Semicolon);
        Ok(DefaultExport::Expression(expr))
    }

    fn default_function_has_name(&self, async_prefix: bool) -> bool {
        let mut offset = if async_prefix { 2 } else { 1 };
        if matches!(
            self.peek_nth(offset).map(|token| &token.kind),
            Some(TokenKind::Star)
        ) {
            offset += 1;
        }
        matches!(
            self.peek_nth(offset).map(|token| &token.kind),
            Some(TokenKind::Identifier(_))
        )
    }

    fn export_specifier_list(&mut self) -> Result<Vec<ExportSpecifier>, ParseError> {
        self.expect(&TokenKind::LeftBrace)?;
        let mut specifiers = Vec::new();
        while !self.at(&TokenKind::RightBrace) {
            let (local, local_span) = self.module_export_name()?;
            let (exported, end) = if self.match_contextual_keyword("as") {
                let (name, span) = self.module_export_name()?;
                (name, span.end)
            } else {
                (local.clone(), local_span.end)
            };
            specifiers.push(ExportSpecifier {
                local,
                exported,
                span: Span::new(local_span.start, end),
            });
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RightBrace)?;
        Ok(specifiers)
    }

    /// Parses a `ModuleExportName`: an identifier name or a string literal.
    fn module_export_name(&mut self) -> Result<(ModuleExportName, Span), ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) => Ok((ModuleExportName::Identifier(name), token.span)),
            TokenKind::String(value) => Ok((ModuleExportName::String(value), token.span)),
            kind => {
                if let Some(name) = crate::expression::keyword_property_name(&kind) {
                    Ok((ModuleExportName::Identifier(name.to_owned()), token.span))
                } else {
                    Err(ParseError {
                        message: "expected an import/export name".to_owned(),
                        span: token.span,
                    })
                }
            }
        }
    }

    /// Parses a plain binding identifier, returning the name and span.
    fn binding_name(&mut self) -> Result<(String, Span), ParseError> {
        let token = self.advance();
        let TokenKind::Identifier(name) = token.kind else {
            return Err(ParseError {
                message: "expected a binding identifier".to_owned(),
                span: token.span,
            });
        };
        self.check_binding_identifier(&name, token.span)?;
        Ok((name, token.span))
    }

    fn expect_contextual(&mut self, keyword: &str) -> Result<(), ParseError> {
        if self.match_contextual_keyword(keyword) {
            Ok(())
        } else {
            let token = self.peek().expect("parser should always have eof token");
            Err(ParseError {
                message: format!("expected `{keyword}`"),
                span: token.span,
            })
        }
    }

    fn try_string_literal(&mut self) -> Option<StringLiteral> {
        if let Some(token) = self.peek() {
            if let TokenKind::String(value) = &token.kind {
                let value = value.clone();
                let span = token.span;
                self.advance();
                return Some(StringLiteral { value, span });
            }
        }
        None
    }

    fn expect_string_literal(&mut self) -> Result<StringLiteral, ParseError> {
        self.try_string_literal().ok_or_else(|| {
            let token = self.peek().expect("parser should always have eof token");
            ParseError {
                message: "expected a module specifier string".to_owned(),
                span: token.span,
            }
        })
    }

    /// Consumes an optional trailing `;` after a module specifier and returns
    /// the span end to record for the declaration.
    fn finish_module_specifier(&mut self, source_span: Span) -> usize {
        let end = source_span.end;
        self.match_kind(&TokenKind::Semicolon);
        end
    }

    /// Span of the most recently consumed token.
    fn previous_span(&self) -> Span {
        self.tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
    }
}

struct StringLiteral {
    value: String,
    span: Span,
}

/// Reports whether a statement is a declaration that may follow `export`
/// directly (`var`/`let`/`const`, `function`, or `class`).
fn is_exportable_declaration(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::VarDecl { .. } | Stmt::FunctionDecl { .. } | Stmt::ClassDecl { .. }
    )
}
