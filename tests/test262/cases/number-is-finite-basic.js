// Derived from: test/built-ins/Number/isFinite/finite-numbers.js
if (!Number.isFinite(10)) {
  throw "expected finite number to return true";
}
if (Number.isFinite(Infinity)) {
  throw "expected Infinity to return false";
}
if (Number.isFinite(NaN)) {
  throw "expected NaN to return false";
}
if (Number.isFinite("10")) {
  throw "expected non-number argument to return false";
}
