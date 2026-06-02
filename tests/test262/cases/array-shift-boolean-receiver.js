// Derived from: test/built-ins/Array/prototype/shift/S15.4.4.9_A2_T2.js
if (Array.prototype.shift.call(false) !== undefined) {
  throw "expected shift on a boolean receiver to return undefined";
}
