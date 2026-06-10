use crate::span::Span;
use crate::statement::{BindingElement, BindingPattern, Stmt};

mod assignment;
mod literal;
mod member;
mod object;
mod operator;

pub use assignment::{
    AssignmentOp, AssignmentTarget, AssignmentTargetElement, AssignmentTargetProperty,
};
pub use literal::Literal;
pub use member::MemberProperty;
pub use object::{ObjectProperty, ObjectPropertyKey, ObjectPropertyKind};
pub use operator::{BinaryOp, UnaryOp, UpdateOp};

/// Function formal parameters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionParams {
    /// Positional parameter bindings with optional default initializers.
    pub positional: Vec<BindingElement>,
    /// Optional rest parameter binding.
    pub rest: Option<Box<BindingPattern>>,
}

impl FunctionParams {
    /// Creates a simple parameter list from plain identifier names.
    #[must_use]
    pub fn positional(positional: Vec<String>) -> Self {
        Self {
            positional: positional
                .into_iter()
                .map(|name| BindingElement::identifier(name, Span::new(0, 0)))
                .collect(),
            rest: None,
        }
    }

    /// Creates a parameter list.
    #[must_use]
    pub fn new(positional: Vec<BindingElement>, rest: Option<BindingPattern>) -> Self {
        Self {
            positional,
            rest: rest.map(Box::new),
        }
    }

    /// Returns all declared parameter names, including rest bindings.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.named_spans()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    /// Returns all declared parameter names paired with their identifier spans.
    #[must_use]
    pub fn named_spans(&self) -> Vec<(String, Span)> {
        let mut names = Vec::new();
        for element in &self.positional {
            names.extend(element.binding.named_spans());
        }
        if let Some(rest) = &self.rest {
            names.extend(rest.named_spans());
        }
        names
    }

    /// Returns the number of positional and rest parameter positions.
    #[must_use]
    pub fn binding_count(&self) -> usize {
        self.positional.len() + usize::from(self.rest.is_some())
    }

    /// Returns true when there are no positional or rest parameters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.positional.is_empty() && self.rest.is_none()
    }

    /// Returns the JavaScript function length.
    #[must_use]
    pub fn length(&self) -> usize {
        self.positional
            .iter()
            .position(|element| element.default.is_some())
            .unwrap_or(self.positional.len())
    }

    /// Returns true when every parameter is a plain identifier without a
    /// default and there is no rest parameter.
    #[must_use]
    pub fn is_simple(&self) -> bool {
        self.rest.is_none()
            && self.positional.iter().all(|element| {
                element.default.is_none()
                    && matches!(element.binding, BindingPattern::Identifier { .. })
            })
    }

    /// Returns the default initializer for a positional parameter.
    #[must_use]
    pub fn default_at(&self, index: usize) -> Option<&Expr> {
        self.positional
            .get(index)
            .and_then(|element| element.default.as_ref())
    }
}

/// An expression node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// A literal value.
    Literal(Literal),
    /// An array literal.
    Array {
        /// Array elements, including elisions and spread elements.
        elements: Vec<ArrayElement>,
        /// Source span.
        span: Span,
    },
    /// An object literal.
    Object {
        /// Object properties.
        properties: Vec<ObjectProperty>,
        /// Source span.
        span: Span,
    },
    /// A comma-separated sequence expression.
    Sequence {
        /// Expressions evaluated from left to right.
        expressions: Vec<Expr>,
        /// Source span.
        span: Span,
    },
    /// A unary expression.
    Unary {
        /// Unary operator.
        op: UnaryOp,
        /// Operand expression.
        argument: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// A binary expression.
    Binary {
        /// Left-hand expression.
        left: Box<Expr>,
        /// Binary operator.
        op: BinaryOp,
        /// Right-hand expression.
        right: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// An untagged template literal with cooked string segments.
    Template {
        /// Cooked string segments. This always has one more entry than `expressions`.
        parts: Vec<String>,
        /// Substitution expressions evaluated left to right.
        expressions: Vec<Expr>,
        /// Source span.
        span: Span,
    },
    /// A tagged template call expression.
    TaggedTemplate {
        /// Tag expression called with the template object.
        tag: Box<Expr>,
        /// Cooked string segments. This always has one more entry than `expressions`.
        cooked: Vec<String>,
        /// Raw string segments. This always has one more entry than `expressions`.
        raw: Vec<String>,
        /// Substitution expressions evaluated left to right.
        expressions: Vec<Expr>,
        /// Source span.
        span: Span,
    },
    /// A conditional expression.
    Conditional {
        /// Test expression.
        test: Box<Expr>,
        /// Expression evaluated when the test is truthy.
        consequent: Box<Expr>,
        /// Expression evaluated when the test is falsy.
        alternate: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// An assignment expression.
    Assignment {
        /// Assigned target.
        target: AssignmentTarget,
        /// Assignment operator.
        op: AssignmentOp,
        /// Assigned value.
        value: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// An update expression.
    Update {
        /// Updated target.
        target: AssignmentTarget,
        /// Update operator.
        op: UpdateOp,
        /// Whether this is a prefix update.
        prefix: bool,
        /// Source span.
        span: Span,
    },
    /// A call expression.
    Call {
        /// Callee expression.
        callee: Box<Expr>,
        /// Arguments, including spread arguments.
        arguments: Vec<CallArgument>,
        /// Source span.
        span: Span,
    },
    /// A constructor call expression.
    New {
        /// Constructor expression.
        callee: Box<Expr>,
        /// Arguments, including spread arguments.
        arguments: Vec<CallArgument>,
        /// Source span.
        span: Span,
    },
    /// A function expression.
    Function {
        /// Optional function name.
        name: Option<String>,
        /// Formal parameters.
        params: FunctionParams,
        /// Function body statements.
        body: Vec<Stmt>,
        /// Whether the function can be called with `new`.
        constructable: bool,
        /// Whether `this` is captured lexically from the creation environment.
        lexical_this: bool,
        /// Whether `arguments` resolves through the lexical environment.
        lexical_arguments: bool,
        /// Source span.
        span: Span,
    },
    /// A member access expression.
    Member {
        /// Object expression.
        object: Box<Expr>,
        /// Property expression or name.
        property: MemberProperty,
        /// Source span.
        span: Span,
    },
    /// A `this` expression.
    This { span: Span },
    /// An identifier reference.
    Identifier { name: String, span: Span },
}

/// An array literal element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArrayElement {
    /// An omitted element in a sparse array literal.
    Elision,
    /// A normal element expression.
    Expr(Expr),
    /// A spread element expression.
    Spread(Expr),
}

/// A function or constructor call argument.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallArgument {
    /// A normal argument expression.
    Expr(Expr),
    /// A spread argument expression.
    Spread(Expr),
}

impl Expr {
    /// Returns the source span for this expression.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Literal(literal) => literal.span(),
            Self::Array { span, .. }
            | Self::Object { span, .. }
            | Self::Sequence { span, .. }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::Template { span, .. }
            | Self::TaggedTemplate { span, .. }
            | Self::Conditional { span, .. }
            | Self::Assignment { span, .. }
            | Self::Update { span, .. }
            | Self::Call { span, .. }
            | Self::New { span, .. }
            | Self::Function { span, .. }
            | Self::Member { span, .. }
            | Self::This { span }
            | Self::Identifier { span, .. } => *span,
        }
    }
}
