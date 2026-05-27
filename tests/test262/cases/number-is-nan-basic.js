// Derived from: test/built-ins/Number/isNaN/nan.js
if (!Number.isNaN(NaN)) {
  throw "expected Number.isNaN(NaN) to return true";
}
if (Number.isNaN(0)) {
  throw "expected Number.isNaN(0) to return false";
}
if (Number.isNaN("NaN")) {
  throw "expected Number.isNaN to not coerce strings";
}
