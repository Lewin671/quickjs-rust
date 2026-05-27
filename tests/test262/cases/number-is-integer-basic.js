// Derived from: test/built-ins/Number/isInteger/integers.js
if (!Number.isInteger(478)) {
  throw "expected integer to return true";
}
if (!Number.isInteger(-0)) {
  throw "expected -0 to be an integer";
}
if (Number.isInteger(6.75)) {
  throw "expected non-integer to return false";
}
if (Number.isInteger(Infinity)) {
  throw "expected Infinity to return false";
}
