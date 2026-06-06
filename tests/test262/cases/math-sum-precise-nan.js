// Derived from: test/built-ins/Math/sumPrecise/sum-is-NaN.js
if (Math.sumPrecise([NaN]) === Math.sumPrecise([NaN])) {
  throw "expected Math.sumPrecise NaN input to return NaN";
}
if (Math.sumPrecise([Infinity, -Infinity]) === Math.sumPrecise([Infinity, -Infinity])) {
  throw "expected Math.sumPrecise mixed infinities to return NaN";
}
if (Math.sumPrecise([-Infinity, Infinity]) === Math.sumPrecise([-Infinity, Infinity])) {
  throw "expected Math.sumPrecise mixed infinities to return NaN";
}
