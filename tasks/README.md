# Tasks

Use these as agent-sized work items. Keep each task independently testable.

## T001: Lexer Coverage

Expand `qjs-lexer` to cover comments, template literals, regex ambiguity notes,
and more punctuators. Add focused token tests.

## T002: Parser Expressions

Add precedence parsing for unary, multiplicative, comparison, equality, logical,
assignment, and comma expressions.

## T003: Statements

Add blocks, variable declarations, `if`, loops, `return`, and function
declarations. Keep AST additions separate from runtime behavior.

## T004: Runtime Values

Introduce JavaScript value types, basic coercion, lexical environments, and
structured runtime errors.

## T005: Conformance Harness

Create a small test harness that can run local fixtures and later import slices
of Test262.
