// Derived from: test/built-ins/Math/f16round/value-conversion.js
if (Math.f16round(1.00048828125) !== 1) {
  throw "expected half-way binary16 value to round to even";
}
if (Math.f16round(1.0009765625) !== 1.0009765625) {
  throw "expected exactly representable binary16 value";
}
if (Math.f16round(65519) !== 65504) {
  throw "expected finite values near max binary16 to round down";
}
if (Math.f16round(65520) !== Infinity) {
  throw "expected binary16 overflow threshold to round to Infinity";
}
if (1 / Math.f16round(-0) !== -Infinity) {
  throw "expected f16round to preserve negative zero";
}
if (Math.f16round(NaN) === Math.f16round(NaN)) {
  throw "expected f16round to preserve NaN";
}
