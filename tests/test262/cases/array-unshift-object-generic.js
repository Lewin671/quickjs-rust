// Derived from: test/built-ins/Array/prototype/unshift/S15.4.4.13_A2_T1.js
var object = { 0: 1, 1: 2, length: 2 };
var result = Array.prototype.unshift.call(object, -1, 0);
if (result !== 4) {
  throw "expected generic unshift to return new length";
}
if (object.length !== 4) {
  throw "expected generic unshift to update length";
}
if (object[0] !== -1 || object[1] !== 0 || object[2] !== 1 || object[3] !== 2) {
  throw "expected generic unshift to prepend values and move existing elements";
}
