// Derived from: test/built-ins/Math/sumPrecise/sum.js
if (Math.sumPrecise([1, 2, 3]) !== 6) {
  throw "expected Math.sumPrecise to sum finite numbers";
}
if (Math.sumPrecise([1e308, -1e308]) !== 0) {
  throw "expected Math.sumPrecise to cancel large finite numbers";
}
if (Math.sumPrecise([1e30, 0.1, -1e30]) !== 0.1) {
  throw "expected Math.sumPrecise to preserve small addends";
}
if (
  Math.sumPrecise([1e308, 1e308, 0.1, 0.1, 1e30, 0.1, -1e30, -1e308, -1e308]) !==
  0.30000000000000004
) {
  throw "expected Math.sumPrecise to round the exact sum";
}
