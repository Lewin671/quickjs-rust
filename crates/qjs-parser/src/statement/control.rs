use qjs_ast::{
    AssignmentTarget, BindingPattern, CatchClause, ForInLeft, ForInit, Span, Stmt, SwitchCase,
    VarKind,
};
use qjs_lexer::TokenKind;

use crate::helpers::{stmt_end, var_kind};
use crate::{ParseError, Parser};

fn disallowed_declaration(stmt: &Stmt) -> Option<(&'static str, Span)> {
    match stmt {
        Stmt::ClassDecl { span, .. } => Some(("class declarations", *span)),
        Stmt::VarDecl {
            kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
            span,
            ..
        } => Some(("lexical declarations", *span)),
        Stmt::FunctionDecl {
            is_generator: true,
            is_async: true,
            span,
            ..
        } => Some(("async generator declarations", *span)),
        Stmt::FunctionDecl {
            is_async: true,
            span,
            ..
        } => Some(("async function declarations", *span)),
        Stmt::FunctionDecl {
            is_generator: true,
            span,
            ..
        } => Some(("generator declarations", *span)),
        Stmt::Labelled { body, .. } => disallowed_declaration(body),
        _ => None,
    }
}

/// CatchParameter early errors (13.15.1): its BoundNames must contain no
/// duplicates, and must not collide with the catch block's lexically declared
/// names (`let`/`const`/`class`/`function` declarations directly in the block).
/// A plain `catch (x) { var x; }` remains legal.
fn validate_catch_parameter(param: &BindingPattern, body: &[Stmt]) -> Result<(), ParseError> {
    let bound = param.named_spans();
    for (index, (name, _)) in bound.iter().enumerate() {
        for (candidate, span) in &bound[index + 1..] {
            if candidate == name {
                return Err(ParseError {
                    message: format!("duplicate catch parameter binding `{name}`"),
                    span: *span,
                });
            }
        }
    }

    let block_lexical: Vec<(String, Span)> = body
        .iter()
        .flat_map(|stmt| match stmt {
            Stmt::ClassDecl { name, span, .. } | Stmt::FunctionDecl { name, span, .. } => {
                vec![(name.clone(), *span)]
            }
            Stmt::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                declarations,
                ..
            } => declarations
                .iter()
                .flat_map(|declarator| declarator.binding.named_spans())
                .collect(),
            _ => Vec::new(),
        })
        .collect();

    for (name, _) in &bound {
        if let Some((_, span)) = block_lexical
            .iter()
            .find(|(candidate, _)| candidate == name)
        {
            return Err(ParseError {
                message: format!("catch parameter `{name}` conflicts with a lexical declaration"),
                span: *span,
            });
        }
    }
    Ok(())
}

fn disallowed_iteration_body(stmt: &Stmt) -> Option<(&'static str, Span)> {
    if let Some(r) = disallowed_declaration(stmt) {
        return Some(r);
    }
    match stmt {
        Stmt::FunctionDecl { span, .. } => Some(("function declarations", *span)),
        Stmt::Labelled { body, .. } => disallowed_iteration_body(body),
        _ => None,
    }
}

fn disallowed_if_body(stmt: &Stmt, strict: bool) -> Option<(&'static str, Span)> {
    if let Some(r) = disallowed_declaration(stmt) {
        return Some(r);
    }
    match stmt {
        Stmt::FunctionDecl {
            is_generator: false,
            is_async: false,
            span,
            ..
        } if strict => Some(("function declarations in strict mode", *span)),
        Stmt::Labelled { body, .. } => is_labelled_function(body),
        _ => None,
    }
}

fn is_labelled_function(stmt: &Stmt) -> Option<(&'static str, Span)> {
    match stmt {
        Stmt::FunctionDecl { span, .. } => Some(("labelled function declarations", *span)),
        Stmt::Labelled { body, .. } => is_labelled_function(body),
        _ => disallowed_declaration(stmt),
    }
}

pub(super) fn disallowed_labelled_body(stmt: &Stmt, strict: bool) -> Option<(&'static str, Span)> {
    match stmt {
        Stmt::ClassDecl { span, .. } => Some(("class declarations", *span)),
        Stmt::VarDecl {
            kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
            span,
            ..
        } => Some(("lexical declarations", *span)),
        Stmt::FunctionDecl {
            is_generator: true,
            is_async: true,
            span,
            ..
        } => Some(("async generator declarations", *span)),
        Stmt::FunctionDecl {
            is_async: true,
            span,
            ..
        } => Some(("async function declarations", *span)),
        Stmt::FunctionDecl {
            is_generator: true,
            span,
            ..
        } => Some(("generator declarations", *span)),
        Stmt::FunctionDecl {
            is_generator: false,
            is_async: false,
            span,
            ..
        } if strict => Some(("function declarations in strict mode", *span)),
        _ => None,
    }
}

fn check_iteration_body(stmt: &Stmt, context: &str) -> Result<(), ParseError> {
    if let Some((description, span)) = disallowed_iteration_body(stmt) {
        return Err(ParseError {
            message: format!("{description} are not allowed as the body of {context}"),
            span,
        });
    }
    Ok(())
}

fn validate_switch_lexical_names(cases: &[SwitchCase], strict: bool) -> Result<(), ParseError> {
    use std::collections::HashMap;
    let mut lexical: HashMap<String, Span> = HashMap::new();
    let mut var_names: HashMap<String, Span> = HashMap::new();
    let mut sloppy_fns: HashMap<String, Span> = HashMap::new();
    for case in cases {
        for stmt in &case.consequent {
            for (name, span) in lexical_names_of(stmt, strict) {
                if lexical.contains_key(&name) {
                    return Err(ParseError {
                        message: format!("duplicate lexical declaration '{name}'"),
                        span,
                    });
                }
                if var_names.contains_key(&name) || sloppy_fns.contains_key(&name) {
                    return Err(ParseError {
                        message: format!("'{name}' conflicts with lexical declaration"),
                        span,
                    });
                }
                lexical.insert(name, span);
            }
            for (name, span) in var_names_of(stmt) {
                if lexical.contains_key(&name) || sloppy_fns.contains_key(&name) {
                    return Err(ParseError {
                        message: format!("variable '{name}' conflicts with lexical declaration"),
                        span,
                    });
                }
                var_names.insert(name, span);
            }
            if !strict {
                for (name, span) in sloppy_function_names_of(stmt) {
                    if lexical.contains_key(&name) || var_names.contains_key(&name) {
                        return Err(ParseError {
                            message: format!(
                                "function '{name}' conflicts with lexical declaration"
                            ),
                            span,
                        });
                    }
                    sloppy_fns.insert(name, span);
                }
            }
        }
    }
    Ok(())
}

fn lexical_names_of(stmt: &Stmt, strict: bool) -> Vec<(String, Span)> {
    match stmt {
        Stmt::ClassDecl { name, span, .. } => vec![(name.clone(), *span)],
        Stmt::VarDecl {
            kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
            declarations,
            span,
            ..
        } => declarations
            .iter()
            .flat_map(|d| d.binding.names().into_iter().map(|n| (n.to_owned(), *span)))
            .collect(),
        Stmt::FunctionDecl {
            name,
            is_generator,
            is_async,
            span,
            ..
        } if *is_generator || *is_async || strict => vec![(name.clone(), *span)],
        _ => vec![],
    }
}

fn var_names_of(stmt: &Stmt) -> Vec<(String, Span)> {
    match stmt {
        Stmt::VarDecl {
            kind: VarKind::Var,
            declarations,
            span,
            ..
        } => declarations
            .iter()
            .flat_map(|d| d.binding.names().into_iter().map(|n| (n.to_owned(), *span)))
            .collect(),
        _ => vec![],
    }
}

fn var_declared_names_of(stmt: &Stmt) -> Vec<(String, Span)> {
    let mut names = Vec::new();
    collect_var_declared_names(stmt, &mut names);
    names
}

fn collect_var_declared_names(stmt: &Stmt, names: &mut Vec<(String, Span)>) {
    match stmt {
        Stmt::VarDecl {
            kind: VarKind::Var,
            declarations,
            ..
        } => {
            for declarator in declarations {
                names.extend(declarator.binding.named_spans());
            }
        }
        Stmt::VarDecl {
            kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
            ..
        } => {}
        Stmt::Block { body, .. }
        | Stmt::Try {
            block: body,
            handler: None,
            finalizer: None,
            ..
        } => {
            for stmt in body {
                collect_var_declared_names(stmt, names);
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_var_declared_names(consequent, names);
            if let Some(alternate) = alternate {
                collect_var_declared_names(alternate, names);
            }
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForIn { body, .. }
        | Stmt::ForOf { body, .. }
        | Stmt::Labelled { body, .. }
        | Stmt::With { body, .. } => collect_var_declared_names(body, names),
        Stmt::Switch { cases, .. } => {
            for case in cases {
                for stmt in &case.consequent {
                    collect_var_declared_names(stmt, names);
                }
            }
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            for stmt in block {
                collect_var_declared_names(stmt, names);
            }
            if let Some(handler) = handler {
                for stmt in &handler.body {
                    collect_var_declared_names(stmt, names);
                }
            }
            if let Some(finalizer) = finalizer {
                for stmt in finalizer {
                    collect_var_declared_names(stmt, names);
                }
            }
        }
        Stmt::FunctionDecl { .. }
        | Stmt::ClassDecl { .. }
        | Stmt::Expr(_)
        | Stmt::Return { .. }
        | Stmt::Throw { .. }
        | Stmt::Debugger { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Empty
        | Stmt::ModuleDecl(_) => {}
    }
}

fn sloppy_function_names_of(stmt: &Stmt) -> Vec<(String, Span)> {
    match stmt {
        Stmt::FunctionDecl {
            name,
            is_generator: false,
            is_async: false,
            span,
            ..
        } => vec![(name.clone(), *span)],
        _ => vec![],
    }
}

impl Parser {
    pub(super) fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::If)?;
        self.expect(&TokenKind::LeftParen)?;
        let test = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        let consequent = self.statement()?;
        if let Some((desc, span)) = disallowed_if_body(&consequent, self.strict) {
            return Err(ParseError {
                message: format!("{desc} are not allowed as the body of an if statement"),
                span,
            });
        }
        let alternate = if self.match_kind(&TokenKind::Else) {
            let alt = self.statement()?;
            if let Some((desc, span)) = disallowed_if_body(&alt, self.strict) {
                return Err(ParseError {
                    message: format!("{desc} are not allowed as the body of an else clause"),
                    span,
                });
            }
            Some(Box::new(alt))
        } else {
            None
        };
        let end = alternate
            .as_deref()
            .map_or_else(|| stmt_end(&consequent), stmt_end);
        Ok(Stmt::If {
            test,
            consequent: Box::new(consequent),
            alternate,
            span: Span::new(start, end),
        })
    }

    pub(super) fn while_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LeftParen)?;
        let test = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        let body = self.statement()?;
        check_iteration_body(&body, "a while loop")?;
        let end = stmt_end(&body);
        Ok(Stmt::While {
            test,
            body: Box::new(body),
            span: Span::new(start, end),
        })
    }

    pub(super) fn do_while_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Do)?;
        let body = self.statement()?;
        check_iteration_body(&body, "a do-while loop")?;
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LeftParen)?;
        let test = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::DoWhile {
            body: Box::new(body),
            test,
            span: Span::new(start, end),
        })
    }

    pub(super) fn for_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::For)?;
        // `for await (... of ...)` iterates with the async protocol and is only
        // valid inside an async function. `await` is the contextual keyword
        // here, recognized only in async context.
        let is_await = if self.in_async
            && matches!(self.peek(), Some(token) if matches!(&token.kind, TokenKind::Identifier(name) if name == "await"))
        {
            let await_token = self.advance();
            Some(await_token.span)
        } else {
            None
        };
        self.expect(&TokenKind::LeftParen)?;
        let for_head_using = if self.at_for_using_of_identifier_head() {
            None
        } else {
            self.using_declaration_kind()
        };
        if self.at(&TokenKind::Var)
            || self.at(&TokenKind::Let)
            || self.at(&TokenKind::Const)
            || for_head_using.is_some()
        {
            let var_head_start = self.cursor;
            let head_start = self.peek().map_or(start, |token| token.span.start);
            let (kind, is_let_token) = if let Some(using_kind) = for_head_using {
                if using_kind == VarKind::AwaitUsing {
                    self.advance(); // `await`
                }
                self.advance(); // `using`
                (using_kind, false)
            } else {
                let kind_token = self.advance();
                let kind = var_kind(&kind_token.kind).expect("token should be declaration kind");
                (kind, kind_token.kind == TokenKind::Let)
            };
            if let Some(binding) = self.for_head_binding(kind) {
                let init = if (kind == VarKind::Var || kind.is_using())
                    && matches!(binding, BindingPattern::Identifier { .. })
                    && self.match_kind(&TokenKind::Equal)
                {
                    Some(self.expression_no_in()?)
                } else {
                    None
                };
                let left_span = Span::new(head_start, binding.span().end);
                if self.match_kind(&TokenKind::In) {
                    if kind.is_using() {
                        return Err(ParseError {
                            message: "`using` declarations may not appear in a for-in loop head"
                                .to_owned(),
                            span: left_span,
                        });
                    }
                    if self.strict && init.is_some() {
                        return Err(ParseError {
                            message:
                                "for-in variable declarations cannot have initializers in strict mode"
                                    .to_owned(),
                            span: self
                                .peek()
                                .expect("parser should always have eof token")
                                .span,
                        });
                    }
                    let left = ForInLeft::VarDecl {
                        kind,
                        binding,
                        init,
                        span: left_span,
                    };
                    return self.finish_for_in_of(start, left, ForKind::In, is_await);
                }
                if init.is_none() && self.match_contextual_keyword("of") {
                    let left = ForInLeft::VarDecl {
                        kind,
                        binding,
                        init: None,
                        span: left_span,
                    };
                    return self.finish_for_in_of(start, left, ForKind::Of, is_await);
                }
            }
            self.cursor = var_head_start;
            if is_let_token && !self.strict && !self.at(&TokenKind::Semicolon) {
                let cursor = self.cursor;
                if let Ok(left) = self.assignment_pattern() {
                    if self.match_kind(&TokenKind::In) {
                        return self.finish_for_in_of(
                            start,
                            ForInLeft::Target(left),
                            ForKind::In,
                            is_await,
                        );
                    }
                    if self.match_contextual_keyword("of") {
                        reject_for_of_lhs_keyword(&left, "let")?;
                        return self.finish_for_in_of(
                            start,
                            ForInLeft::Target(left),
                            ForKind::Of,
                            is_await,
                        );
                    }
                }
                self.cursor = cursor;
            }
        } else if !self.at(&TokenKind::Semicolon) {
            let cursor = self.cursor;
            if let Ok(left) = self.assignment_pattern() {
                if self.match_kind(&TokenKind::In) {
                    return self.finish_for_in_of(
                        start,
                        ForInLeft::Target(left),
                        ForKind::In,
                        is_await,
                    );
                }
                if !self.at_async_of_arrow_after_lhs(&left) && self.match_contextual_keyword("of") {
                    // The leading-`async` restriction is `for-of` only; `for
                    // await (async of x)` is valid (the for-await grammar has no
                    // such lookahead restriction).
                    if is_await.is_none() {
                        reject_for_of_lhs_keyword(&left, "async")?;
                    }
                    return self.finish_for_in_of(
                        start,
                        ForInLeft::Target(left),
                        ForKind::Of,
                        is_await,
                    );
                }
            }
            self.cursor = cursor;
        }

        // A `for await` head that did not resolve to a for-of loop is a syntax
        // error: `for await` requires the async iteration `of` form.
        if let Some(await_span) = is_await {
            return Err(ParseError {
                message: "`for await` requires a `for await (... of ...)` loop".to_owned(),
                span: await_span,
            });
        }

        let init = if self.match_kind(&TokenKind::Semicolon) {
            None
        } else if self.at(&TokenKind::Var)
            || self.at_let_lexical_declaration()
            || self.at(&TokenKind::Const)
            || (!self.at_for_using_of_identifier_head() && self.using_declaration_kind().is_some())
        {
            let init = self.for_variable_declaration()?;
            self.expect(&TokenKind::Semicolon)?;
            Some(init)
        } else {
            let init = self.expression_no_in()?;
            self.expect(&TokenKind::Semicolon)?;
            Some(ForInit::Expr(init))
        };

        let test = if self.at(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&TokenKind::Semicolon)?;

        let update = if self.at(&TokenKind::RightParen) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&TokenKind::RightParen)?;
        let body = self.statement()?;
        validate_for_head(&init, &body)?;
        check_iteration_body(&body, "a for loop")?;
        let end = stmt_end(&body);
        Ok(Stmt::For {
            init,
            test,
            update,
            body: Box::new(body),
            span: Span::new(start, end),
        })
    }

    pub(super) fn with_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        // `with` is a SyntaxError in strict-mode code (ECMA-262 14.11.1).
        if self.strict {
            return Err(ParseError {
                message: "`with` statements are not allowed in strict mode".to_owned(),
                span: Span::new(start, start + "with".len()),
            });
        }
        self.expect(&TokenKind::With)?;
        self.expect(&TokenKind::LeftParen)?;
        let object = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        let body = self.statement()?;
        if let Some((desc, span)) = disallowed_iteration_body(&body) {
            return Err(ParseError {
                message: format!("{desc} are not allowed as the body of a with statement"),
                span,
            });
        }
        let end = stmt_end(&body);
        Ok(Stmt::With {
            object,
            body: Box::new(body),
            span: Span::new(start, end),
        })
    }

    pub(super) fn switch_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Switch)?;
        self.expect(&TokenKind::LeftParen)?;
        let discriminant = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        self.expect(&TokenKind::LeftBrace)?;

        let mut cases = Vec::new();
        let mut seen_default = false;
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            let clause_start = self
                .peek()
                .expect("parser should always have eof token")
                .span
                .start;
            let test = if self.match_kind(&TokenKind::Case) {
                let test = self.expression()?;
                self.expect(&TokenKind::Colon)?;
                Some(test)
            } else if self.match_kind(&TokenKind::Default) {
                if seen_default {
                    return Err(ParseError {
                        message: "switch statement cannot have multiple default clauses".to_owned(),
                        span: Span::new(clause_start, clause_start + "default".len()),
                    });
                }
                seen_default = true;
                self.expect(&TokenKind::Colon)?;
                None
            } else {
                let token = self.peek().expect("parser should always have eof token");
                return Err(ParseError {
                    message: "expected switch case or default clause".to_owned(),
                    span: token.span,
                });
            };

            let mut consequent = Vec::new();
            while !self.at(&TokenKind::Case)
                && !self.at(&TokenKind::Default)
                && !self.at(&TokenKind::RightBrace)
                && !self.at(&TokenKind::Eof)
            {
                let stmt = self.statement()?;
                if let Stmt::VarDecl {
                    kind: VarKind::Using | VarKind::AwaitUsing,
                    span,
                    ..
                } = &stmt
                {
                    return Err(ParseError {
                        message: "`using` declarations are not allowed directly in switch clauses"
                            .to_owned(),
                        span: *span,
                    });
                }
                consequent.push(stmt);
            }
            let end = consequent
                .last()
                .map_or_else(|| self.tokens[self.cursor - 1].span.end, stmt_end);
            cases.push(SwitchCase {
                test,
                consequent,
                span: Span::new(clause_start, end),
            });
        }

        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        validate_switch_lexical_names(&cases, self.strict)?;
        Ok(Stmt::Switch {
            discriminant,
            cases,
            span: Span::new(start, end),
        })
    }

    fn at_for_using_of_identifier_head(&self) -> bool {
        let (Some(using_token), Some(first_of_token), Some(second_of_token)) =
            (self.peek_nth(0), self.peek_nth(1), self.peek_nth(2))
        else {
            return false;
        };
        matches!(&using_token.kind, TokenKind::Identifier(name) if name == "using")
            && matches!(&first_of_token.kind, TokenKind::Identifier(name) if name == "of")
            && matches!(&second_of_token.kind, TokenKind::Identifier(name) if name == "of")
            && !self.has_line_terminator_between(using_token.span.end, first_of_token.span.start)
            && !self
                .has_line_terminator_between(first_of_token.span.end, second_of_token.span.start)
    }

    fn at_let_lexical_declaration(&self) -> bool {
        if !self.at(&TokenKind::Let) {
            return false;
        }
        matches!(
            self.peek_nth(1).map(|token| &token.kind),
            Some(
                TokenKind::Identifier(_)
                    | TokenKind::Let
                    | TokenKind::LeftBracket
                    | TokenKind::LeftBrace
            )
        )
    }

    fn at_async_of_arrow_after_lhs(&self, left: &AssignmentTarget) -> bool {
        matches!(
            left,
            AssignmentTarget::Identifier {
                name,
                parenthesized: false,
                ..
            } if name == "async"
        ) && matches!(
            (self.peek(), self.peek_nth(1)),
            (Some(token), Some(next))
                if matches!(&token.kind, TokenKind::Identifier(name) if name == "of")
                    && next.kind == TokenKind::Arrow
        )
    }

    pub(super) fn try_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Try)?;
        let block = self.block_statements()?;
        let handler = if self.at(&TokenKind::Catch) {
            Some(self.catch_clause()?)
        } else {
            None
        };
        let finalizer = if self.match_kind(&TokenKind::Finally) {
            Some(self.block_statements()?)
        } else {
            None
        };

        if handler.is_none() && finalizer.is_none() {
            let token = self.peek().expect("parser should always have eof token");
            return Err(ParseError {
                message: "try statement requires catch or finally".to_owned(),
                span: token.span,
            });
        }

        let end = finalizer
            .as_ref()
            .and_then(|body| body.last().map(stmt_end))
            .or_else(|| handler.as_ref().map(|handler| handler.span.end))
            .or_else(|| block.last().map(stmt_end))
            .unwrap_or(start + "try".len());
        Ok(Stmt::Try {
            block,
            handler,
            finalizer,
            span: Span::new(start, end),
        })
    }

    fn catch_clause(&mut self) -> Result<CatchClause, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Catch)?;
        let param = if self.match_kind(&TokenKind::LeftParen) {
            let param = self.binding_pattern()?;
            self.expect(&TokenKind::RightParen)?;
            Some(param)
        } else {
            None
        };
        let body = self.block_statements()?;
        if let Some(param) = &param {
            validate_catch_parameter(param, &body)?;
        }
        let end = body.last().map_or(start + "catch".len(), stmt_end);
        Ok(CatchClause {
            param,
            body,
            span: Span::new(start, end),
        })
    }

    /// Parses a for-in/for-of declaration binding, returning `None` (with
    /// the cursor rewound) when the tokens do not form a binding pattern.
    fn for_head_binding(&mut self, kind: VarKind) -> Option<BindingPattern> {
        let cursor = self.cursor;
        if self.at(&TokenKind::LeftBracket) || self.at(&TokenKind::LeftBrace) {
            match self.binding_pattern() {
                Ok(pattern) => return Some(pattern),
                Err(_) => {
                    self.cursor = cursor;
                    return None;
                }
            }
        }
        let token = self.peek()?;
        let name = match &token.kind {
            TokenKind::Identifier(name) => name.clone(),
            TokenKind::Let if kind == VarKind::Var && !self.strict => "let".to_owned(),
            _ => return None,
        };
        if self.strict && matches!(name.as_str(), "eval" | "arguments") {
            return None;
        }
        let span = token.span;
        self.advance();
        Some(BindingPattern::Identifier { name, span })
    }

    fn finish_for_in_of(
        &mut self,
        start: usize,
        left: ForInLeft,
        kind: ForKind,
        is_await: Option<Span>,
    ) -> Result<Stmt, ParseError> {
        // `for await` is only valid with the `of` form.
        if kind == ForKind::In
            && let Some(await_span) = is_await
        {
            return Err(ParseError {
                message: "`for await` may not be used with a for-in loop".to_owned(),
                span: await_span,
            });
        }
        let right = if kind == ForKind::In {
            self.expression()?
        } else {
            self.assignment()?
        };
        self.expect(&TokenKind::RightParen)?;
        let body = self.statement()?;
        let loop_kind = match kind {
            ForKind::In => "a for-in loop",
            ForKind::Of => "a for-of loop",
        };
        validate_for_in_of_head(&left, &body)?;
        check_iteration_body(&body, loop_kind)?;
        let end = stmt_end(&body);
        let span = Span::new(start, end);
        let body = Box::new(body);
        Ok(match kind {
            ForKind::In => Stmt::ForIn {
                left,
                right,
                body,
                span,
            },
            ForKind::Of => Stmt::ForOf {
                left,
                right,
                body,
                is_await: is_await.is_some(),
                span,
            },
        })
    }
}

/// Rejects a bare `let`/`async` identifier as the left-hand side of a `for-of`
/// loop. The ForBinding grammar forbids `[lookahead = let]` and a leading
/// `async`, so `for (let of x)` / `for (async of y)` are early errors — but the
/// parenthesized (`for ((let) of x)`) and member (`for (let.p of x)`) forms stay
/// valid, and `for-in` is unaffected.
fn reject_for_of_lhs_keyword(left: &AssignmentTarget, keyword: &str) -> Result<(), ParseError> {
    if let AssignmentTarget::Identifier {
        name,
        span,
        parenthesized: false,
    } = left
        && name == keyword
    {
        return Err(ParseError {
            message: format!("`{keyword}` is not allowed as the left-hand side of a `for-of` loop"),
            span: *span,
        });
    }
    Ok(())
}

fn validate_for_head(init: &Option<ForInit>, body: &Stmt) -> Result<(), ParseError> {
    let Some(ForInit::VarDecl {
        kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
        declarations,
        ..
    }) = init
    else {
        return Ok(());
    };

    let mut bound_names = Vec::new();
    for declaration in declarations {
        bound_names.extend(declaration.binding.named_spans());
    }
    validate_lexical_for_head_bound_names(&bound_names, body, "for loop")
}

fn validate_for_in_of_head(left: &ForInLeft, body: &Stmt) -> Result<(), ParseError> {
    let ForInLeft::VarDecl {
        kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
        binding,
        ..
    } = left
    else {
        return Ok(());
    };

    let bound_names = binding.named_spans();
    validate_lexical_for_head_bound_names(&bound_names, body, "for-in/of loop")
}

fn validate_lexical_for_head_bound_names(
    bound_names: &[(String, Span)],
    body: &Stmt,
    loop_kind: &str,
) -> Result<(), ParseError> {
    for (index, (name, _)) in bound_names.iter().enumerate() {
        for (candidate, span) in &bound_names[index + 1..] {
            if candidate == name {
                return Err(ParseError {
                    message: format!("duplicate {loop_kind} binding `{name}`"),
                    span: *span,
                });
            }
        }
    }

    for (name, _) in bound_names {
        for (var_name, span) in var_declared_names_of(body) {
            if &var_name == name {
                return Err(ParseError {
                    message: format!(
                        "{loop_kind} body declaration `{var_name}` conflicts with lexical head"
                    ),
                    span,
                });
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ForKind {
    In,
    Of,
}
