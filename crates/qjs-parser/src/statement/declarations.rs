use qjs_ast::{ForInit, Span, Stmt, VarDeclarator, VarKind};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    pub(super) fn variable_declaration(&mut self) -> Result<Stmt, ParseError> {
        let ForInit::VarDecl {
            kind,
            declarations,
            span,
        } = self.for_variable_declaration()?
        else {
            unreachable!("for variable declaration helper always returns VarDecl");
        };
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::VarDecl {
            kind,
            declarations,
            span,
        })
    }

    pub(super) fn for_variable_declaration(&mut self) -> Result<ForInit, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let kind = if self.match_kind(&TokenKind::Var) {
            VarKind::Var
        } else if self.match_kind(&TokenKind::Let) {
            VarKind::Let
        } else {
            self.expect(&TokenKind::Const)?;
            VarKind::Const
        };

        let declarations = self.variable_declarator_list(kind)?;
        let end = declarations.last().map_or(start, |decl| decl.span.end);
        Ok(ForInit::VarDecl {
            kind,
            declarations,
            span: Span::new(start, end),
        })
    }

    fn variable_declarator_list(
        &mut self,
        kind: VarKind,
    ) -> Result<Vec<VarDeclarator>, ParseError> {
        let mut declarations = Vec::new();
        loop {
            let name_token = self.advance();
            let TokenKind::Identifier(name) = name_token.kind else {
                return Err(ParseError {
                    message: "expected binding identifier".to_owned(),
                    span: name_token.span,
                });
            };

            let init = if self.match_kind(&TokenKind::Equal) {
                Some(self.assignment()?)
            } else {
                if kind == VarKind::Const {
                    return Err(ParseError {
                        message: "const declarations require an initializer".to_owned(),
                        span: name_token.span,
                    });
                }
                None
            };
            let end = init
                .as_ref()
                .map_or(name_token.span.end, |expr| expr.span().end);
            declarations.push(VarDeclarator {
                name,
                init,
                span: Span::new(name_token.span.start, end),
            });

            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        Ok(declarations)
    }
}
