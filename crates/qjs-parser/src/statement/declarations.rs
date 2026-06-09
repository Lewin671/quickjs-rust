use qjs_ast::{
    BindingElement, BindingPattern, ForInit, ObjectBindingProperty, Span, Stmt, VarDeclarator,
    VarKind,
};
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
            let binding = self.binding_pattern()?;

            let init = if self.match_kind(&TokenKind::Equal) {
                Some(self.assignment()?)
            } else {
                if !matches!(binding, BindingPattern::Identifier { .. }) {
                    return Err(ParseError {
                        message: "destructuring declarations require an initializer".to_owned(),
                        span: binding.span(),
                    });
                }
                if kind == VarKind::Const {
                    return Err(ParseError {
                        message: "const declarations require an initializer".to_owned(),
                        span: binding.span(),
                    });
                }
                None
            };
            let end = init
                .as_ref()
                .map_or(binding.span().end, |expr| expr.span().end);
            let start = binding.span().start;
            declarations.push(VarDeclarator {
                binding,
                init,
                span: Span::new(start, end),
            });

            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        Ok(declarations)
    }

    fn binding_pattern(&mut self) -> Result<BindingPattern, ParseError> {
        if self.at(&TokenKind::LeftBracket) {
            return self.array_binding_pattern();
        }
        if self.at(&TokenKind::LeftBrace) {
            return self.object_binding_pattern();
        }

        let token = self.advance();
        let TokenKind::Identifier(name) = token.kind else {
            return Err(ParseError {
                message: "expected binding identifier".to_owned(),
                span: token.span,
            });
        };
        Ok(BindingPattern::Identifier {
            name,
            span: token.span,
        })
    }

    fn binding_element(&mut self) -> Result<BindingElement, ParseError> {
        let binding = self.binding_pattern()?;
        let default = if self.match_kind(&TokenKind::Equal) {
            Some(self.assignment()?)
        } else {
            None
        };
        let end = default
            .as_ref()
            .map_or(binding.span().end, |expr| expr.span().end);
        let start = binding.span().start;
        Ok(BindingElement {
            binding,
            default,
            span: Span::new(start, end),
        })
    }

    fn array_binding_pattern(&mut self) -> Result<BindingPattern, ParseError> {
        let start = self.advance().span.start;
        let mut elements = Vec::new();
        while !self.at(&TokenKind::RightBracket) {
            if self.match_kind(&TokenKind::Comma) {
                elements.push(None);
                continue;
            }

            elements.push(Some(self.binding_element()?));
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        if !self.at(&TokenKind::RightBracket) {
            let token = self.peek().expect("parser should always have eof token");
            return Err(ParseError {
                message: "expected `RightBracket`".to_owned(),
                span: token.span,
            });
        }
        let end = self.advance().span.end;
        Ok(BindingPattern::Array {
            elements,
            span: Span::new(start, end),
        })
    }

    fn object_binding_pattern(&mut self) -> Result<BindingPattern, ParseError> {
        let start = self.advance().span.start;
        let mut properties = Vec::new();
        while !self.at(&TokenKind::RightBrace) {
            let key_token = self.advance();
            let TokenKind::Identifier(key) = key_token.kind else {
                return Err(ParseError {
                    message: "expected binding property name".to_owned(),
                    span: key_token.span,
                });
            };

            let binding = if self.match_kind(&TokenKind::Colon) {
                self.binding_pattern()?
            } else {
                BindingPattern::Identifier {
                    name: key.clone(),
                    span: key_token.span,
                }
            };
            let default = if self.match_kind(&TokenKind::Equal) {
                Some(self.assignment()?)
            } else {
                None
            };
            let end = default
                .as_ref()
                .map_or(binding.span().end, |expr| expr.span().end);
            properties.push(ObjectBindingProperty {
                key,
                binding,
                default,
                span: Span::new(key_token.span.start, end),
            });

            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        if !self.at(&TokenKind::RightBrace) {
            let token = self.peek().expect("parser should always have eof token");
            return Err(ParseError {
                message: "expected `RightBrace`".to_owned(),
                span: token.span,
            });
        }
        let end = self.advance().span.end;
        Ok(BindingPattern::Object {
            properties,
            span: Span::new(start, end),
        })
    }
}
