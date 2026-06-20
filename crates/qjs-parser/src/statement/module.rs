use std::collections::HashSet;

use qjs_ast::{
    DEFAULT_EXPORT_BINDING, DefaultExport, ExportDecl, ExportSpecifier, Expr, ImportAttributes,
    ImportDecl, ImportSpecifier, ModuleDecl, ModuleExportName, Span, Stmt,
};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

const SURROGATE_ESCAPE_SENTINEL_BASE: u32 = 0xF0000;

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
            let (attributes, end) = self.finish_module_specifier(source.span)?;
            return Ok(ImportDecl {
                specifiers: Vec::new(),
                source: source.value,
                attributes,
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
        let (attributes, end) = self.finish_module_specifier(source.span)?;
        validate_import_bound_names(&specifiers)?;
        Ok(ImportDecl {
            specifiers,
            source: source.value,
            attributes,
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
                self.check_binding_identifier(name, imported.1)?;
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
            let (_, end) = self.finish_module_specifier(source.span)?;
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
            let (_, end) = self.finish_module_specifier(self.previous_span())?;
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
            return Self::anonymous_default_function_export(expr);
        }
        if self.at(&TokenKind::Function) {
            if self.default_function_has_name(false) {
                let stmt = self.statement()?;
                return Ok(DefaultExport::Declaration(Box::new(stmt)));
            }
            let start = self.advance().span.start;
            let expr = self.function_expression(start)?;
            return Self::anonymous_default_function_export(expr);
        }
        if self.at(&TokenKind::Class) {
            let expr = self.assignment()?;
            return Ok(DefaultExport::Expression(expr));
        }
        let expr = self.assignment()?;
        self.consume_module_declaration_terminator(expr.span().end)?;
        Ok(DefaultExport::Expression(expr))
    }

    fn anonymous_default_function_export(expr: Expr) -> Result<DefaultExport, ParseError> {
        let Expr::Function {
            name: None,
            params,
            body,
            is_generator,
            is_async,
            span,
            ..
        } = expr
        else {
            return Ok(DefaultExport::Expression(expr));
        };
        Ok(DefaultExport::Declaration(Box::new(Stmt::FunctionDecl {
            name: DEFAULT_EXPORT_BINDING.to_owned(),
            params,
            body,
            is_generator,
            is_async,
            span,
        })))
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
            TokenKind::String(value) => {
                if !is_well_formed_module_export_name(&value) {
                    return Err(ParseError {
                        message: "module export name string must be well-formed Unicode".to_owned(),
                        span: token.span,
                    });
                }
                Ok((ModuleExportName::String(value), token.span))
            }
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

    /// Consumes optional import attributes, then requires a
    /// module-declaration terminator and returns the parsed attributes plus the
    /// span end to record.
    fn finish_module_specifier(
        &mut self,
        source_span: Span,
    ) -> Result<(ImportAttributes, usize), ParseError> {
        let mut end = source_span.end;
        let attributes = self.consume_import_attributes()?;
        if attributes.1 {
            end = self.previous_span().end;
        }
        self.consume_module_declaration_terminator(end)?;
        Ok((attributes.0, end))
    }

    fn consume_module_declaration_terminator(
        &mut self,
        declaration_end: usize,
    ) -> Result<(), ParseError> {
        if self.match_kind(&TokenKind::Semicolon) {
            return Ok(());
        }
        if self.at(&TokenKind::RightBrace) || self.at(&TokenKind::Eof) {
            return Ok(());
        }
        let next = self.peek().expect("parser should always have eof token");
        if self.has_line_terminator_between(declaration_end, next.span.start) {
            return Ok(());
        }
        Err(ParseError {
            message: "expected `;` or newline after module declaration".to_owned(),
            span: next.span,
        })
    }

    fn consume_import_attributes(&mut self) -> Result<(ImportAttributes, bool), ParseError> {
        if !matches!(
            self.peek(),
            Some(token) if !token.had_escape && token.kind == TokenKind::With
        ) {
            return Ok((ImportAttributes::default(), false));
        }
        self.advance(); // `with`
        self.expect_kind(TokenKind::LeftBrace)?;
        if self.match_kind(&TokenKind::RightBrace) {
            return Ok((ImportAttributes::default(), true));
        }
        let mut module_type = None;
        let mut keys = HashSet::new();
        loop {
            let key = self.attribute_key()?;
            if !keys.insert(key.clone()) {
                let span = self.previous_span();
                return Err(ParseError {
                    message: "duplicate import attribute key".to_owned(),
                    span,
                });
            }
            self.expect_kind(TokenKind::Colon)?;
            let value = self.expect_string_literal()?;
            if key == "type" {
                module_type = Some(value.value);
            }
            if self.match_kind(&TokenKind::Comma) {
                if self.match_kind(&TokenKind::RightBrace) {
                    break;
                }
                continue;
            }
            self.expect_kind(TokenKind::RightBrace)?;
            break;
        }
        Ok((ImportAttributes { module_type }, true))
    }

    fn attribute_key(&mut self) -> Result<String, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) if !token.had_escape => Ok(name),
            TokenKind::String(value) => Ok(value),
            _ => Err(ParseError {
                message: "expected an import attribute key".to_owned(),
                span: token.span,
            }),
        }
    }

    fn expect_kind(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.match_kind(&kind) {
            Ok(())
        } else {
            let token = self.peek().expect("parser should always have eof token");
            Err(ParseError {
                message: format!("expected {kind:?}"),
                span: token.span,
            })
        }
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

fn validate_import_bound_names(specifiers: &[ImportSpecifier]) -> Result<(), ParseError> {
    let mut names = Vec::new();
    for specifier in specifiers {
        let (local, span) = import_local_name_and_span(specifier);
        if names.contains(&local) {
            return Err(ParseError {
                message: format!("duplicate import binding `{local}`"),
                span,
            });
        }
        names.push(local);
    }
    Ok(())
}

fn import_local_name_and_span(specifier: &ImportSpecifier) -> (&str, Span) {
    match specifier {
        ImportSpecifier::Default { local, span }
        | ImportSpecifier::Namespace { local, span }
        | ImportSpecifier::Named { local, span, .. } => (local, *span),
    }
}

pub(super) fn validate_module_static_semantics(body: &[Stmt]) -> Result<(), ParseError> {
    validate_module_exported_names(body)?;
    validate_module_local_export_bindings(body)?;
    validate_module_top_level_declarations(body)
}

fn validate_module_exported_names(body: &[Stmt]) -> Result<(), ParseError> {
    let mut exported_names: Vec<(String, Span)> = Vec::new();
    for stmt in body {
        let Stmt::ModuleDecl(ModuleDecl::Export(export)) = stmt else {
            continue;
        };
        collect_exported_names(export, &mut exported_names);
    }
    for (index, (name, _)) in exported_names.iter().enumerate() {
        for (candidate, span) in &exported_names[index + 1..] {
            if candidate == name {
                return Err(ParseError {
                    message: format!("duplicate exported name `{name}`"),
                    span: *span,
                });
            }
        }
    }
    Ok(())
}

fn collect_exported_names(export: &ExportDecl, out: &mut Vec<(String, Span)>) {
    match export {
        ExportDecl::Named { specifiers, .. } => {
            out.extend(
                specifiers
                    .iter()
                    .map(|specifier| (specifier.exported.as_str().to_owned(), specifier.span)),
            );
        }
        ExportDecl::All {
            exported: Some(name),
            span,
            ..
        } => out.push((name.as_str().to_owned(), *span)),
        ExportDecl::All { exported: None, .. } => {}
        ExportDecl::Default { span, .. } => out.push(("default".to_owned(), *span)),
        ExportDecl::Declaration { declaration, .. } => {
            out.extend(declared_names_and_spans(declaration));
        }
    }
}

fn validate_module_local_export_bindings(body: &[Stmt]) -> Result<(), ParseError> {
    let declared_names = module_declared_names(body);
    for stmt in body {
        let Stmt::ModuleDecl(ModuleDecl::Export(ExportDecl::Named {
            specifiers,
            source: None,
            ..
        })) = stmt
        else {
            continue;
        };
        for specifier in specifiers {
            if matches!(specifier.local, ModuleExportName::String(_)) {
                return Err(ParseError {
                    message: "local export binding must be an identifier".to_owned(),
                    span: specifier.span,
                });
            }
            if !declared_names
                .iter()
                .any(|name| name == specifier.local.as_str())
            {
                return Err(ParseError {
                    message: format!(
                        "exported binding `{}` is not declared in this module",
                        specifier.local.as_str()
                    ),
                    span: specifier.span,
                });
            }
        }
    }
    Ok(())
}

fn is_well_formed_module_export_name(value: &str) -> bool {
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        let Some(unit) = surrogate_escape_code_unit(ch) else {
            continue;
        };
        if (0xD800..=0xDBFF).contains(&unit) {
            if !matches!(
                chars
                    .peek()
                    .and_then(|next| surrogate_escape_code_unit(*next)),
                Some(0xDC00..=0xDFFF)
            ) {
                return false;
            }
            chars.next();
        } else {
            return false;
        }
    }
    true
}

fn surrogate_escape_code_unit(character: char) -> Option<u16> {
    let code = character as u32;
    if (SURROGATE_ESCAPE_SENTINEL_BASE..SURROGATE_ESCAPE_SENTINEL_BASE + 0x800).contains(&code) {
        Some((0xD800 + code - SURROGATE_ESCAPE_SENTINEL_BASE) as u16)
    } else {
        None
    }
}

fn module_declared_names(body: &[Stmt]) -> Vec<String> {
    let mut names = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::ModuleDecl(ModuleDecl::Import(import)) => {
                names.extend(
                    import
                        .specifiers
                        .iter()
                        .map(|specifier| import_local_name_and_span(specifier).0.to_owned()),
                );
            }
            Stmt::VarDecl { declarations, .. } => {
                for declarator in declarations {
                    names.extend(declarator.binding.names());
                }
            }
            Stmt::FunctionDecl { name, .. } | Stmt::ClassDecl { name, .. } => {
                names.push(name.clone());
            }
            Stmt::ModuleDecl(ModuleDecl::Export(ExportDecl::Declaration {
                declaration, ..
            })) => {
                names.extend(
                    declared_names_and_spans(declaration)
                        .into_iter()
                        .map(|(name, _)| name),
                );
            }
            Stmt::ModuleDecl(ModuleDecl::Export(ExportDecl::Default {
                declaration: DefaultExport::Declaration(declaration),
                ..
            })) => {
                names.extend(
                    declared_names_and_spans(declaration)
                        .into_iter()
                        .map(|(name, _)| name),
                );
            }
            _ => {}
        }
    }
    names
}

fn validate_module_top_level_declarations(body: &[Stmt]) -> Result<(), ParseError> {
    let mut lexical_names: Vec<(String, Span)> = Vec::new();
    let mut var_names: Vec<(String, Span)> = Vec::new();
    let mut function_names: Vec<(String, Span)> = Vec::new();
    for stmt in body {
        collect_module_top_level_declaration_names(
            stmt,
            &mut lexical_names,
            &mut var_names,
            &mut function_names,
        );
    }

    for (index, (name, _)) in lexical_names.iter().enumerate() {
        for (candidate, span) in &lexical_names[index + 1..] {
            if candidate == name {
                return Err(ParseError {
                    message: format!("duplicate lexical declaration `{name}`"),
                    span: *span,
                });
            }
        }
    }

    for (lexical_name, _) in &lexical_names {
        for (var_name, span) in &var_names {
            if var_name == lexical_name {
                return Err(ParseError {
                    message: format!(
                        "declaration `{var_name}` conflicts with a lexical declaration"
                    ),
                    span: *span,
                });
            }
        }
    }

    for (index, (name, span)) in function_names.iter().enumerate() {
        if function_names[index + 1..]
            .iter()
            .any(|(candidate, _)| candidate == name)
        {
            return Err(ParseError {
                message: format!("duplicate lexical declaration `{name}`"),
                span: *span,
            });
        }
        if lexical_names.iter().any(|(candidate, _)| candidate == name)
            || var_names.iter().any(|(candidate, _)| candidate == name)
        {
            return Err(ParseError {
                message: format!("declaration `{name}` conflicts with a lexical declaration"),
                span: *span,
            });
        }
    }
    Ok(())
}

fn collect_module_top_level_declaration_names(
    stmt: &Stmt,
    lexical_names: &mut Vec<(String, Span)>,
    var_names: &mut Vec<(String, Span)>,
    function_names: &mut Vec<(String, Span)>,
) {
    match stmt {
        Stmt::VarDecl {
            kind: qjs_ast::VarKind::Var,
            declarations,
            ..
        } => {
            for declaration in declarations {
                var_names.extend(declaration.binding.named_spans());
            }
        }
        Stmt::VarDecl { declarations, .. } => {
            for declaration in declarations {
                lexical_names.extend(declaration.binding.named_spans());
            }
        }
        Stmt::ClassDecl { name, span, .. } => lexical_names.push((name.clone(), *span)),
        Stmt::FunctionDecl {
            name,
            is_generator,
            is_async,
            span,
            ..
        } => {
            if *is_generator || *is_async {
                lexical_names.push((name.clone(), *span));
            } else {
                function_names.push((name.clone(), *span));
            }
        }
        Stmt::ModuleDecl(ModuleDecl::Export(ExportDecl::Declaration { declaration, .. })) => {
            collect_module_top_level_declaration_names(
                declaration,
                lexical_names,
                var_names,
                function_names,
            );
        }
        Stmt::ModuleDecl(ModuleDecl::Export(ExportDecl::Default {
            declaration: DefaultExport::Declaration(declaration),
            ..
        })) => collect_module_top_level_declaration_names(
            declaration,
            lexical_names,
            var_names,
            function_names,
        ),
        _ => {}
    }
}

fn declared_names_and_spans(stmt: &Stmt) -> Vec<(String, Span)> {
    match stmt {
        Stmt::VarDecl { declarations, .. } => declarations
            .iter()
            .flat_map(|declaration| declaration.binding.named_spans())
            .collect(),
        Stmt::FunctionDecl { name, span, .. } | Stmt::ClassDecl { name, span, .. } => {
            vec![(name.clone(), *span)]
        }
        _ => Vec::new(),
    }
}
