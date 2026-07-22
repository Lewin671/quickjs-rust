mod classes;
mod control;
mod declarations;
mod functions;
mod module;

pub(crate) use functions::{duplicate_parameter_span, is_strict_reserved_word};
mod simple;

use qjs_ast::{ForInLeft, ForInit, Script, Span, Stmt, VarKind};
use qjs_lexer::TokenKind;

use crate::{Goal, ParseError, Parser};

impl Parser {
    pub(crate) fn parse_script(&mut self) -> Result<Script, ParseError> {
        self.strict = self.strict || self.directive_prologue_is_strict(self.cursor);
        let mut body = Vec::new();
        while !self.at(&TokenKind::Eof) {
            if self.goal == Goal::Module {
                body.push(self.module_item()?);
            } else {
                body.push(self.statement_list_item()?);
            }
        }
        // A `using`/`await using` declaration is a Syntax Error at the top level
        // of a Script (and eval, which uses the Script goal): it must be
        // contained in a block, function/generator body, for-head, or class
        // static block. Module top level permits it.
        if self.goal == Goal::Script {
            for stmt in &body {
                if let Stmt::VarDecl {
                    kind: VarKind::Using | VarKind::AwaitUsing,
                    span,
                    ..
                } = stmt
                {
                    return Err(ParseError {
                        message:
                            "`using` declarations are not allowed at the top level of a script"
                                .to_owned(),
                        span: *span,
                    });
                }
            }
        }
        // At the top level of a Module, function declarations are
        // LexicallyDeclaredNames (unlike a Script, where Annex B treats them as
        // var-like), so a top-level `function f(){}` clashing with a `var f`
        // is an early error — the same rule that applies inside a block.
        validate_statement_list_declarations(&body, self.goal == Goal::Module, self.strict)?;
        if self.goal == Goal::Module {
            module::validate_module_static_semantics(&body)?;
        }
        validate_statement_list_labels(&body)?;
        // Any private-name reference that never resolved to an enclosing class
        // is a syntax error.
        if let Some(reference) = self.pending_private_refs.first() {
            return Err(ParseError {
                message: format!(
                    "private name `#{}` is not declared in scope",
                    reference.name
                ),
                span: reference.span,
            });
        }
        Ok(Script {
            body,
            source: self.source.as_str().into(),
            source_is_wtf16: self.source_is_wtf16,
        })
    }

    pub(super) fn statement_list_item(&mut self) -> Result<Stmt, ParseError> {
        self.statement_with_lexical_declarations(true)
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        self.statement_with_lexical_declarations(false)
    }

    fn statement_with_lexical_declarations(
        &mut self,
        allow_lexical_declarations: bool,
    ) -> Result<Stmt, ParseError> {
        if self.match_kind(&TokenKind::Semicolon) {
            return Ok(Stmt::Empty);
        }

        if self.at(&TokenKind::LeftBrace) {
            return self.block_statement();
        }

        if self.at(&TokenKind::If) {
            return self.if_statement();
        }

        if self.at(&TokenKind::While) {
            return self.while_statement();
        }

        if self.at(&TokenKind::Do) {
            return self.do_while_statement();
        }

        if self.at(&TokenKind::For) {
            return self.for_statement();
        }

        if self.at(&TokenKind::Switch) {
            return self.switch_statement();
        }

        if self.at(&TokenKind::With) {
            return self.with_statement();
        }

        if self.at(&TokenKind::Try) {
            return self.try_statement();
        }

        if self.at(&TokenKind::Function) {
            return self.function_declaration();
        }

        if let Some(error) = self.escaped_async_function_keyword_error() {
            return Err(error);
        }

        // `async function` (with no line terminator between) is an async
        // function declaration. `async` followed by anything else is a plain
        // identifier expression statement.
        if self.at_async_function_keyword() {
            let async_token = self.advance();
            return self.function_declaration_with_async(async_token.span.start, true);
        }

        if self.at(&TokenKind::Class) {
            return self.class_declaration();
        }

        if self.at(&TokenKind::Return) {
            return self.return_statement();
        }

        if self.at(&TokenKind::Throw) {
            return self.throw_statement();
        }

        if self.at(&TokenKind::Debugger) {
            return Ok(self.debugger_statement());
        }

        if self.at(&TokenKind::Break) {
            return Ok(self.break_or_continue_statement(TokenKind::Break));
        }

        if self.at(&TokenKind::Continue) {
            return Ok(self.break_or_continue_statement(TokenKind::Continue));
        }

        if self.starts_labelled_statement() {
            return self.labelled_statement();
        }

        if self.at(&TokenKind::Var) || self.at(&TokenKind::Const) {
            return self.variable_declaration();
        }
        if self.at(&TokenKind::Let) && self.let_is_declaration_start(allow_lexical_declarations) {
            return self.variable_declaration();
        }
        // `using x = ...` / `await using x = ...` are contextual declarations.
        if self.using_declaration_kind().is_some() {
            return self.variable_declaration();
        }
        if self.at(&TokenKind::Let)
            && matches!(self.peek_nth(1), Some(token) if token.kind == TokenKind::LeftBracket)
        {
            let token = self.peek().expect("parser should always have eof token");
            return Err(ParseError {
                message: "expression statements may not start with `let [`".to_owned(),
                span: token.span,
            });
        }

        let expr = self.expression()?;
        let end = expr.span().end;
        self.consume_statement_terminator(end)?;
        Ok(Stmt::Expr(expr))
    }

    pub(crate) fn consume_statement_terminator(
        &mut self,
        statement_end: usize,
    ) -> Result<(), ParseError> {
        if self.match_kind(&TokenKind::Semicolon) {
            return Ok(());
        }
        if self.at(&TokenKind::RightBrace) || self.at(&TokenKind::Eof) {
            return Ok(());
        }
        let next = self
            .peek()
            .expect("parser should always have eof token")
            .clone();
        if self.has_line_terminator_between(statement_end, next.span.start) {
            return Ok(());
        }
        Err(ParseError {
            message: "expected `;` or newline after statement".to_owned(),
            span: next.span,
        })
    }

    fn let_is_declaration_start(&self, allow_line_terminator_before_identifier: bool) -> bool {
        if self.strict {
            return true;
        }
        let Some(next) = self.peek_nth(1) else {
            return false;
        };
        match &next.kind {
            TokenKind::LeftBracket | TokenKind::LeftBrace => {
                let let_end = self.tokens[self.cursor].span.end;
                let next_start = next.span.start;
                let between = &self.source[let_end..next_start];
                !between.contains('\n')
                    && !between.contains('\r')
                    && !between.contains('\u{2028}')
                    && !between.contains('\u{2029}')
            }
            TokenKind::Identifier(_) => {
                let let_end = self.tokens[self.cursor].span.end;
                let next_start = next.span.start;
                allow_line_terminator_before_identifier
                    || !self.has_line_terminator_between(let_end, next_start)
            }
            TokenKind::Let => allow_line_terminator_before_identifier,
            _ => false,
        }
    }

    fn starts_labelled_statement(&self) -> bool {
        matches!(self.peek(), Some(token) if matches!(token.kind, TokenKind::Identifier(_)))
            && matches!(self.peek_nth(1), Some(token) if token.kind == TokenKind::Colon)
    }

    fn labelled_statement(&mut self) -> Result<Stmt, ParseError> {
        let label_token = self.advance();
        let TokenKind::Identifier(label) = label_token.kind else {
            unreachable!("caller should check label token")
        };
        // An escaped spelling of a reserved word (e.g. `false`) reaches
        // here as Identifier("false") with had_escape set. Per ECMA-262 12.1,
        // the StringValue of such a token still matches a reserved word, so it
        // cannot serve as a LabelIdentifier.
        if crate::helpers::is_reserved_identifier_name(&label) {
            return Err(ParseError {
                message: format!("`{label}` is a reserved word"),
                span: label_token.span,
            });
        }
        // `await` is reserved as a LabelIdentifier inside an async function and
        // `yield` inside a generator (or in strict mode), so they may not be
        // used as labels there.
        if (self.in_async || self.in_static_block) && label == "await" {
            return Err(ParseError {
                message: "`await` may not be used as a label here".to_owned(),
                span: label_token.span,
            });
        }
        if (self.in_generator || self.strict) && label == "yield" {
            return Err(ParseError {
                message: "`yield` may not be used as a label here".to_owned(),
                span: label_token.span,
            });
        }
        self.expect(&TokenKind::Colon)?;
        let body = self.statement()?;
        if let Some((description, span)) = control::disallowed_labelled_body(&body, self.strict) {
            return Err(ParseError {
                message: format!(
                    "{description} are not allowed as the body of a labelled statement"
                ),
                span,
            });
        }
        let end = crate::helpers::stmt_end(&body);
        Ok(Stmt::Labelled {
            label,
            body: Box::new(body),
            span: Span::new(label_token.span.start, end),
        })
    }

    pub(super) fn block_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::LeftBrace)?;
        let mut body = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            body.push(self.statement_list_item()?);
        }
        validate_statement_list_declarations(&body, true, self.strict)?;
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok(Stmt::Block {
            body,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn block_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect(&TokenKind::LeftBrace)?;
        let previous_strict = self.strict;
        self.strict = self.strict || self.directive_prologue_is_strict(self.cursor);
        let result = (|parser: &mut Self| {
            let mut body = Vec::new();
            while !parser.at(&TokenKind::RightBrace) && !parser.at(&TokenKind::Eof) {
                body.push(parser.statement_list_item()?);
            }
            validate_statement_list_declarations(&body, false, parser.strict)?;
            validate_statement_list_labels(&body)?;
            parser.expect(&TokenKind::RightBrace).map(|()| body)
        })(self);
        self.strict = previous_strict;
        result
    }

    /// Parses a braced block without validating labels. Used for try/catch/finally
    /// blocks where labels from enclosing scopes remain visible.
    pub(crate) fn block_statements(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect(&TokenKind::LeftBrace)?;
        let mut body = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            body.push(self.statement_list_item()?);
        }
        validate_statement_list_declarations(&body, true, self.strict)?;
        self.expect(&TokenKind::RightBrace)?;
        Ok(body)
    }

    fn directive_prologue_is_strict(&self, mut cursor: usize) -> bool {
        while let Some(token) = self.tokens.get(cursor) {
            let TokenKind::String(value) = &token.kind else {
                return false;
            };
            // Only the directive `'use strict'`/`"use strict"` written verbatim
            // enters strict mode; an escape or line continuation computes to the
            // same value but spans more than its 12 source bytes (ES2023 11.2.1).
            if value == "use strict" && token.span.end - token.span.start == 12 {
                return true;
            }
            cursor += 1;
            match self.tokens.get(cursor).map(|t| &t.kind) {
                Some(TokenKind::Semicolon) => cursor += 1,
                Some(_) => {
                    let string_end = token.span.end;
                    let next_start = self.tokens[cursor].span.start;
                    let between = &self.source[string_end..next_start];
                    if !between.contains('\n')
                        && !between.contains('\r')
                        && !between.contains('\u{2028}')
                        && !between.contains('\u{2029}')
                    {
                        return false;
                    }
                }
                None => {}
            }
        }
        false
    }
}

fn validate_statement_list_declarations(
    body: &[Stmt],
    is_block: bool,
    strict: bool,
) -> Result<(), ParseError> {
    let mut lexical_names: Vec<(String, Span)> = Vec::new();
    let mut var_names: Vec<(String, Span)> = Vec::new();
    // Plain (non-async, non-generator) function declarations are tracked apart
    // from `lexical_names`: two of them may share a name (Annex B sloppy
    // semantics), but each still conflicts with a same-named lexical binding,
    // and inside a block also with a same-named `var`.
    let mut block_function_names: Vec<(String, Span)> = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                declarations,
                ..
            } => {
                for declarator in declarations {
                    lexical_names.extend(declarator.binding.named_spans());
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
                    block_function_names.push((name.clone(), *span));
                }
            }
            _ => {}
        }
    }

    // `var` declarations hoist through nested blocks (and other non-function
    // statements) up to this statement list, so a `var` in an inner block is a
    // VarDeclaredName of this block and conflicts with a lexical name declared
    // here. Collect those hoisted names without crossing a function boundary.
    collect_hoisted_var_names(body, &mut var_names);

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

    // Annex B's relaxation that lets two same-named plain function declarations
    // coexist in a block applies only in sloppy mode. In strict code (and a
    // module body) a block-level function declaration is an ordinary
    // LexicallyDeclaredName, so a duplicate is a Syntax Error.
    if strict && is_block {
        for (index, (name, _)) in block_function_names.iter().enumerate() {
            for (candidate, span) in &block_function_names[index + 1..] {
                if candidate == name {
                    return Err(ParseError {
                        message: format!("duplicate lexical declaration `{name}`"),
                        span: *span,
                    });
                }
            }
        }
    }

    // A plain function declaration is a LexicallyDeclaredName, so it clashes with
    // a same-named lexical binding (`let`/`const`/`class`/async/generator
    // function) regardless of scope, and — inside a block, where Annex B's
    // function-as-var relaxation does not apply — also with a same-named `var`.
    for (function_name, span) in &block_function_names {
        let conflicts_with_lexical = lexical_names.iter().any(|(name, _)| name == function_name);
        let conflicts_with_var =
            is_block && var_names.iter().any(|(name, _)| name == function_name);
        if conflicts_with_lexical || conflicts_with_var {
            return Err(ParseError {
                message: format!(
                    "declaration `{function_name}` conflicts with a lexical declaration"
                ),
                span: *span,
            });
        }
    }

    Ok(())
}

/// Collects the `VarDeclaredNames` reachable from `stmts`: every `var` binding
/// declared directly or in a nested statement that does not cross a function
/// boundary (`var` hoists through blocks, `if`/loop bodies, `switch`, `try`,
/// labels, and `with`, but not into nested function or class bodies). Function
/// and class *declarations* are scope boundaries, so their bodies are not
/// descended and their names are not collected here.
fn collect_hoisted_var_names(stmts: &[Stmt], out: &mut Vec<(String, Span)>) {
    for stmt in stmts {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Var,
                declarations,
                ..
            } => {
                for declarator in declarations {
                    out.extend(declarator.binding.named_spans());
                }
            }
            Stmt::Block { body, .. } => collect_hoisted_var_names(body, out),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                collect_hoisted_var_names(std::slice::from_ref(consequent), out);
                if let Some(alternate) = alternate {
                    collect_hoisted_var_names(std::slice::from_ref(alternate), out);
                }
            }
            Stmt::While { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::Labelled { body, .. }
            | Stmt::With { body, .. } => {
                collect_hoisted_var_names(std::slice::from_ref(body), out);
            }
            Stmt::For { init, body, .. } => {
                if let Some(ForInit::VarDecl {
                    kind: VarKind::Var,
                    declarations,
                    ..
                }) = init
                {
                    for declarator in declarations {
                        out.extend(declarator.binding.named_spans());
                    }
                }
                collect_hoisted_var_names(std::slice::from_ref(body), out);
            }
            Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
                if let ForInLeft::VarDecl {
                    kind: VarKind::Var,
                    binding,
                    ..
                } = left
                {
                    out.extend(binding.named_spans());
                }
                collect_hoisted_var_names(std::slice::from_ref(body), out);
            }
            Stmt::Switch { cases, .. } => {
                for case in cases {
                    collect_hoisted_var_names(&case.consequent, out);
                }
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                collect_hoisted_var_names(block, out);
                if let Some(handler) = handler {
                    collect_hoisted_var_names(&handler.body, out);
                }
                if let Some(finalizer) = finalizer {
                    collect_hoisted_var_names(finalizer, out);
                }
            }
            // Function/class declarations are scope boundaries; everything else
            // (expressions, `return`/`throw`/`break`/`continue`, ...) declares
            // no hoisted `var` names.
            _ => {}
        }
    }
}

fn validate_statement_list_labels(body: &[Stmt]) -> Result<(), ParseError> {
    let mut context = LabelContext::default();
    for stmt in body {
        validate_statement_labels(stmt, &mut context)?;
    }
    Ok(())
}

#[derive(Default)]
struct LabelContext {
    break_labels: Vec<String>,
    continue_labels: Vec<String>,
    breakable_depth: usize,
    loop_depth: usize,
}

fn validate_statement_labels(stmt: &Stmt, context: &mut LabelContext) -> Result<(), ParseError> {
    match stmt {
        Stmt::Block { body, .. } => {
            for stmt in body {
                validate_statement_labels(stmt, context)?;
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            validate_statement_labels(consequent, context)?;
            if let Some(alternate) = alternate {
                validate_statement_labels(alternate, context)?;
            }
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForIn { body, .. }
        | Stmt::ForOf { body, .. } => {
            context.breakable_depth += 1;
            context.loop_depth += 1;
            let result = validate_statement_labels(body, context);
            context.loop_depth -= 1;
            context.breakable_depth -= 1;
            result?;
        }
        Stmt::Switch { cases, .. } => {
            context.breakable_depth += 1;
            let result = (|| {
                for case in cases {
                    for stmt in &case.consequent {
                        validate_statement_labels(stmt, context)?;
                    }
                }
                Ok(())
            })();
            context.breakable_depth -= 1;
            result?;
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            for stmt in block {
                validate_statement_labels(stmt, context)?;
            }
            if let Some(handler) = handler {
                for stmt in &handler.body {
                    validate_statement_labels(stmt, context)?;
                }
            }
            if let Some(finalizer) = finalizer {
                for stmt in finalizer {
                    validate_statement_labels(stmt, context)?;
                }
            }
        }
        Stmt::With { body, .. } => validate_statement_labels(body, context)?,
        Stmt::Labelled { label, body, span } => {
            // A label may not be nested inside another statement carrying the
            // same label (`foo: foo: ;` or `foo: { bar: { foo: ; } }`).
            if context
                .break_labels
                .iter()
                .any(|candidate| candidate == label)
            {
                return Err(ParseError {
                    message: format!("duplicate label `{label}`"),
                    span: *span,
                });
            }
            context.break_labels.push(label.clone());
            let is_continue_target = statement_is_iteration_target(body);
            if is_continue_target {
                context.continue_labels.push(label.clone());
            }
            let result = validate_statement_labels(body, context);
            if is_continue_target {
                context.continue_labels.pop();
            }
            context.break_labels.pop();
            result?;
        }
        Stmt::Break {
            label: Some(label),
            span,
        } if !context
            .break_labels
            .iter()
            .any(|candidate| candidate == label) =>
        {
            return Err(ParseError {
                message: format!("undefined break label `{label}`"),
                span: *span,
            });
        }
        Stmt::Break { label: None, span } if context.breakable_depth == 0 => {
            return Err(ParseError {
                message: "`break` has no target".to_owned(),
                span: *span,
            });
        }
        Stmt::Continue { label, span } => match label {
            Some(label)
                if !context
                    .continue_labels
                    .iter()
                    .any(|candidate| candidate == label) =>
            {
                return Err(ParseError {
                    message: format!("undefined continue label `{label}`"),
                    span: *span,
                });
            }
            None if context.loop_depth == 0 => {
                return Err(ParseError {
                    message: "`continue` has no target".to_owned(),
                    span: *span,
                });
            }
            _ => {}
        },
        Stmt::FunctionDecl { .. } | Stmt::ClassDecl { .. } | Stmt::ModuleDecl(_) => {}
        Stmt::Expr(_)
        | Stmt::Return { .. }
        | Stmt::Throw { .. }
        | Stmt::Debugger { .. }
        | Stmt::Break { .. }
        | Stmt::VarDecl { .. }
        | Stmt::Empty => {}
    }
    Ok(())
}

fn statement_is_iteration_target(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::While { .. }
        | Stmt::DoWhile { .. }
        | Stmt::For { .. }
        | Stmt::ForIn { .. }
        | Stmt::ForOf { .. } => true,
        Stmt::Labelled { body, .. } => statement_is_iteration_target(body),
        _ => false,
    }
}
