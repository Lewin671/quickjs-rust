// Derived from: test/built-ins/Function/prototype/bind/length.js
function join(a, b, c) { return "" + a + b + c; }
var bound = join.bind(null, "a");
if (Function.prototype.bind.length !== 1) {
  throw "Function.prototype.bind.length should be 1";
}
if (bound.length !== 2) {
  throw "bound function length should subtract bound arguments";
}
if (bound.propertyIsEnumerable("length")) {
  throw "bound function length should be non-enumerable";
}
