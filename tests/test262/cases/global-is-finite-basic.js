// Derived from: test/built-ins/isFinite/return-true-for-valid-finite-numbers.js
if (!isFinite(10)) {
  throw "expected isFinite(10) to be true";
}
if (!isFinite("10")) {
  throw "expected global isFinite to coerce strings";
}
if (!isFinite(null)) {
  throw "expected global isFinite to coerce null to 0";
}
if (isFinite(Infinity)) {
  throw "expected isFinite(Infinity) to be false";
}
if (isFinite(undefined)) {
  throw "expected isFinite(undefined) to be false";
}
