// Derived from: test/built-ins/Array/prototype/pop/S15.4.4.6_A2_T1.js
let object = { 0: 1, 1: 2, length: 2 };
if (Array.prototype.pop.call(object) !== 2) {
  throw "expected pop to return the last array-like element";
}
if (object.length !== 1) {
  throw "expected pop to decrease array-like length";
}
if (object.hasOwnProperty("1")) {
  throw "expected pop to delete the last own property";
}
