// Derived from: test/built-ins/Array/prototype/shift/S15.4.4.9_A2_T1.js
let object = { 0: 1, 1: 2, 2: 3, length: 3 };
if (Array.prototype.shift.call(object) !== 1) {
  throw "expected shift to return the first array-like element";
}
if (object.length !== 2) {
  throw "expected shift to decrease array-like length";
}
if (object[0] !== 2 || object[1] !== 3) {
  throw "expected shift to move array-like elements left";
}
if (object.hasOwnProperty("2")) {
  throw "expected shift to delete the old last property";
}
