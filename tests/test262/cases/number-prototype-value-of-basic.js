// Derived from: test/built-ins/Number/prototype/valueOf/S15.7.4.4_A1_T01.js
if (Number.prototype.valueOf() !== 0) {
  throw "expected Number.prototype.valueOf() to return 0";
}
if ((10).valueOf() !== 10) {
  throw "expected primitive number valueOf to return the number";
}
if ((new Number(7)).valueOf() !== 7) {
  throw "expected Number object valueOf to use wrapped value";
}
