// Derived from: test/built-ins/Math/log10/Log10-specialVals.js
if (Math.log10(0) !== -Infinity) {
  throw "expected Math.log10(0) to return -Infinity";
}
if (Math.log2(0) !== -Infinity) {
  throw "expected Math.log2(0) to return -Infinity";
}
if (Math.log(-1) === Math.log(-1)) {
  throw "expected Math.log(-1) to return NaN";
}
if (Math.log10(-1) === Math.log10(-1)) {
  throw "expected Math.log10(-1) to return NaN";
}
if (Math.log2(-1) === Math.log2(-1)) {
  throw "expected Math.log2(-1) to return NaN";
}
