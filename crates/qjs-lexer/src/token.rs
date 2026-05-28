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
    /// String literal value.
    String(String),
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
