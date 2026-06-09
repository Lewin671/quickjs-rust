use crate::expression::Expr;
use crate::span::Span;

/// A variable declarator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarDeclarator {
    /// Binding target.
    pub binding: BindingPattern,
    /// Optional initializer.
    pub init: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// A declaration binding pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingPattern {
    /// A single binding identifier.
    Identifier { name: String, span: Span },
    /// An array binding pattern.
    Array {
        /// Elements in source order. `None` represents an elision.
        elements: Vec<Option<BindingElement>>,
        /// Optional trailing rest element.
        rest: Option<Box<BindingPattern>>,
        /// Source span.
        span: Span,
    },
    /// An object binding pattern.
    Object {
        /// Object properties in source order.
        properties: Vec<ObjectBindingProperty>,
        /// Optional trailing rest property binding.
        rest: Option<Box<BindingPattern>>,
        /// Source span.
        span: Span,
    },
}

/// A binding element with an optional default initializer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingElement {
    /// Nested binding pattern.
    pub binding: BindingPattern,
    /// Optional default initializer.
    pub default: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// An object binding property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectBindingProperty {
    /// Literal property key.
    pub key: String,
    /// Nested binding pattern.
    pub binding: BindingPattern,
    /// Optional default initializer.
    pub default: Option<Expr>,
    /// Source span.
    pub span: Span,
}

impl BindingElement {
    /// Creates a binding element for a plain identifier without a default.
    #[must_use]
    pub fn identifier(name: String, span: Span) -> Self {
        Self {
            binding: BindingPattern::Identifier { name, span },
            default: None,
            span,
        }
    }
}

impl BindingPattern {
    /// Returns all names declared by the binding pattern.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.named_spans()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    /// Returns all declared names paired with their identifier spans.
    #[must_use]
    pub fn named_spans(&self) -> Vec<(String, Span)> {
        let mut names = Vec::new();
        self.collect_named_spans(&mut names);
        names
    }

    /// Returns the source span for this pattern.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Identifier { span, .. }
            | Self::Array { span, .. }
            | Self::Object { span, .. } => *span,
        }
    }

    fn collect_named_spans(&self, names: &mut Vec<(String, Span)>) {
        match self {
            Self::Identifier { name, span } => names.push((name.clone(), *span)),
            Self::Array { elements, rest, .. } => {
                for element in elements.iter().flatten() {
                    element.binding.collect_named_spans(names);
                }
                if let Some(rest) = rest {
                    rest.collect_named_spans(names);
                }
            }
            Self::Object {
                properties, rest, ..
            } => {
                for property in properties {
                    property.binding.collect_named_spans(names);
                }
                if let Some(rest) = rest {
                    rest.collect_named_spans(names);
                }
            }
        }
    }
}

/// Variable declaration kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VarKind {
    /// `var`.
    Var,
    /// `let`.
    Let,
    /// `const`.
    Const,
}
