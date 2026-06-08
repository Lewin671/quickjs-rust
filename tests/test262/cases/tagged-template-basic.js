// Derived from: test/language/expressions/template-literal/evaluation-order.js
var calls = 0;
function tag(strings, a, b) {
  calls++;
  if (strings[0] !== "a" || strings[1] !== "b" || strings[2] !== "c") { throw new Error("unexpected cooked segments"); }
  if (strings.raw[0] !== "a" || strings.raw[1] !== "b" || strings.raw[2] !== "c") { throw new Error("unexpected raw segments"); }
  if (a !== 1 || b !== 2) { throw new Error("unexpected substitution arguments"); }
}
tag`a${1}b${2}c`;
if (calls !== 1) { throw new Error("tag must be called once"); }
