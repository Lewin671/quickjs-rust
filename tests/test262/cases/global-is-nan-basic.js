// Derived from: test/built-ins/isNaN/return-true-nan.js
if (!isNaN(NaN)) {
  throw "expected isNaN(NaN) to be true";
}
if (!isNaN("abc")) {
  throw "expected global isNaN to coerce invalid strings";
}
if (isNaN("10")) {
  throw "expected isNaN('10') to be false after coercion";
}
if (isNaN(null)) {
  throw "expected isNaN(null) to be false";
}
