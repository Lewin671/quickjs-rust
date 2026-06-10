use qjs_ast::Span;

/// A token with its source span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    /// Token kind.
    pub kind: TokenKind,
    /// Source span.
    pub span: Span,
}

/// Token categories recognized by the lexer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    /// Identifier text.
    Identifier(String),
    /// Number literal text.
    Number(String),
    /// BigInt literal text without the trailing `n`.
    BigInt(String),
    /// String literal value.
    String(String),
    /// Template literal without substitutions.
    TemplateNoSubstitution(TemplateSegment),
    /// Template literal head before the first substitution.
    TemplateHead(TemplateSegment),
    /// Template literal middle segment between substitutions.
    TemplateMiddle(TemplateSegment),
    /// Template literal tail after the last substitution.
    TemplateTail(TemplateSegment),
    /// Regular expression literal raw pattern and flags.
    RegularExpression { pattern: String, flags: String },
    /// `true`.
    True,
    /// `false`.
    False,
    /// `null`.
    Null,
    /// `this`.
    This,
    /// `var`.
    Var,
    /// `let`.
    Let,
    /// `const`.
    Const,
    /// `if`.
    If,
    /// `else`.
    Else,
    /// `while`.
    While,
    /// `do`.
    Do,
    /// `for`.
    For,
    /// `switch`.
    Switch,
    /// `case`.
    Case,
    /// `default`.
    Default,
    /// `try`.
    Try,
    /// `catch`.
    Catch,
    /// `finally`.
    Finally,
    /// `break`.
    Break,
    /// `continue`.
    Continue,
    /// `function`.
    Function,
    /// `class`.
    Class,
    /// `extends`.
    Extends,
    /// `super`.
    Super,
    /// `return`.
    Return,
    /// `throw`.
    Throw,
    /// `debugger`.
    Debugger,
    /// `typeof`.
    Typeof,
    /// `void`.
    Void,
    /// `in`.
    In,
    /// `delete`.
    Delete,
    /// `new`.
    New,
    /// `instanceof`.
    Instanceof,
    /// `+`.
    Plus,
    /// `++`.
    PlusPlus,
    /// `+=`.
    PlusEqual,
    /// `-`.
    Minus,
    /// `--`.
    MinusMinus,
    /// `-=`.
    MinusEqual,
    /// `=>`.
    Arrow,
    /// `*`.
    Star,
    /// `**`.
    StarStar,
    /// `*=`.
    StarEqual,
    /// `**=`.
    StarStarEqual,
    /// `/`.
    Slash,
    /// `/=`.
    SlashEqual,
    /// `%`.
    Percent,
    /// `%=`.
    PercentEqual,
    /// `=`.
    Equal,
    /// `==`.
    EqualEqual,
    /// `===`.
    EqualEqualEqual,
    /// `!`.
    Bang,
    /// `!=`.
    BangEqual,
    /// `!==`.
    BangEqualEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessEqual,
    /// `<<`.
    LessLess,
    /// `<<=`.
    LessLessEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterEqual,
    /// `>>`.
    GreaterGreater,
    /// `>>=`.
    GreaterGreaterEqual,
    /// `>>>`.
    GreaterGreaterGreater,
    /// `>>>=`.
    GreaterGreaterGreaterEqual,
    /// `&`.
    Ampersand,
    /// `&&`.
    AmpersandAmpersand,
    /// `&=`.
    AmpersandEqual,
    /// `&&=`.
    AmpersandAmpersandEqual,
    /// `|`.
    Pipe,
    /// `||`.
    PipePipe,
    /// `|=`.
    PipeEqual,
    /// `||=`.
    PipePipeEqual,
    /// `^`.
    Caret,
    /// `^=`.
    CaretEqual,
    /// `~`.
    Tilde,
    /// `\`.
    Backslash,
    /// `(`.
    LeftParen,
    /// `)`.
    RightParen,
    /// `{`.
    LeftBrace,
    /// `}`.
    RightBrace,
    /// `[`.
    LeftBracket,
    /// `]`.
    RightBracket,
    /// `,`.
    Comma,
    /// `.`.
    Dot,
    /// `...`.
    DotDotDot,
    /// `:`.
    Colon,
    /// `?`.
    Question,
    /// `??`.
    QuestionQuestion,
    /// `?.`.
    QuestionDot,
    /// `??=`.
    QuestionQuestionEqual,
    /// `;`.
    Semicolon,
    /// End of input.
    Eof,
}

/// Cooked and raw text for a template literal segment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateSegment {
    /// Cooked template value.
    pub cooked: String,
    /// Raw template value.
    pub raw: String,
}
