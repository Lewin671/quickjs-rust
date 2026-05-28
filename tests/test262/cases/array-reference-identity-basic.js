// Derived from: test/language/expressions/strict-equals/S11.9.4_A7.js
let left = [];
let right = left;
if (!(left === right)) {
  throw "expected aliases to reference the same array";
}
if ([] === []) {
  throw "expected distinct array literals to compare unequal";
}
