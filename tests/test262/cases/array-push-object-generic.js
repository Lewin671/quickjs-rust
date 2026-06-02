// Derived from: test/built-ins/Array/prototype/push/S15.4.4.7_A2_T1.js
var object = { 0: 1, length: 1 };
var result = Array.prototype.push.call(object, 2, 3);
if (result !== 3) {
  throw "expected generic push to return new length";
}
if (object.length !== 3) {
  throw "expected generic push to update length";
}
if (object[1] !== 2 || object[2] !== 3) {
  throw "expected generic push to append values";
}
