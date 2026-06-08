// Derived from: test/language/expressions/template-literal/literal-expr-primitive.js
var a = 1;
var b = 2;
if (`${a}${b}` !== "12") { throw new Error("substitutions must be appended"); }
if (`head ${a + b} tail` !== "head 3 tail") { throw new Error("expression substitutions must be evaluated"); }
