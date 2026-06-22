use qjs_ast::{Expr, Literal, Stmt};

/// Byte length of the source text of a Use Strict Directive: the ten
/// characters `use strict` plus the two surrounding quotes.
const USE_STRICT_DIRECTIVE_SOURCE_LEN: usize = 12;

/// Whether a Directive Prologue (the leading string-literal expression
/// statements of `body`) contains a Use Strict Directive.
///
/// The determination is made on the directive's *source text*, not its
/// computed value: a directive is "use strict" only when the characters
/// between its quotes are exactly `use strict`. A directive written with an
/// escape sequence or a line continuation (e.g. `'use str\<LF>ict'`) computes
/// to the string "use strict" but is NOT a Use Strict Directive (ES2023
/// 11.2.1). The literal's span is exactly 12 bytes only when it carries no such
/// escape, so the span length distinguishes the two without re-reading source.
pub(crate) fn is_strict_function_body(body: &[Stmt]) -> bool {
    for stmt in body {
        let Stmt::Expr(Expr::Literal(Literal::String { value, span })) = stmt else {
            return false;
        };
        if value == "use strict" && span.end - span.start == USE_STRICT_DIRECTIVE_SOURCE_LEN {
            return true;
        }
    }
    false
}
