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
        /// Source span.
        span: Span,
    },
    /// An object binding pattern.
    Object {
        /// Object properties in source order.
        properties: Vec<ObjectBindingProperty>,
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

impl BindingPattern {
    /// Returns all names declared by the binding pattern.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let mut names = Vec::new();
        self.collect_names(&mut names);
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

    fn collect_names(&self, names: &mut Vec<String>) {
        match self {
            Self::Identifier { name, .. } => names.push(name.clone()),
            Self::Array { elements, .. } => {
                for element in elements.iter().flatten() {
                    element.binding.collect_names(names);
                }
            }
            Self::Object { properties, .. } => {
                for property in properties {
                    property.binding.collect_names(names);
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
