// Derived from: test/built-ins/Array/prototype/at/index-argument-tointeger.js
var array = [1, 2, 3];

if (array.at(1.9) !== 2) {
  throw "expected at(1.9) to truncate toward zero";
}
if (array.at(-1.9) !== 3) {
  throw "expected at(-1.9) to truncate toward zero before resolving relative index";
}
if (array.at() !== 1) {
  throw "expected omitted index to behave like 0";
}
