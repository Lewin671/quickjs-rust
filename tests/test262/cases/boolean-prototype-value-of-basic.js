// Derived from: test/built-ins/Boolean/prototype/valueOf/S15.6.4.3_A1_T1.js
if (Boolean.prototype.valueOf() !== false) {
  throw "expected Boolean.prototype.valueOf() to return false";
}
if (false.valueOf() !== false) {
  throw "expected false.valueOf() to return false";
}
if ((new Boolean(1)).valueOf() !== true) {
  throw "expected Boolean object valueOf to use wrapped value";
}
if ((new Boolean(0)).valueOf() !== false) {
  throw "expected Boolean object valueOf to coerce constructor argument";
}
