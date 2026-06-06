// Derived from: test/language/expressions/array/spread-mult-literal.js
var xs = [5, ...[6, 7, 8], 9];

if (xs.length !== 5 || xs[0] !== 5 || xs[1] !== 6 || xs[2] !== 7 || xs[3] !== 8 || xs[4] !== 9) {
  throw new Error("expected array spread to append literal elements in order");
}
