// Derived from: test/built-ins/Math/sumPrecise/sum-is-infinite.js
if (Math.sumPrecise([Infinity]) !== Infinity) {
  throw "expected Math.sumPrecise to return Infinity";
}
if (Math.sumPrecise([Infinity, Infinity]) !== Infinity) {
  throw "expected Math.sumPrecise to preserve positive infinity";
}
if (Math.sumPrecise([-Infinity]) !== -Infinity) {
  throw "expected Math.sumPrecise to return -Infinity";
}
if (Math.sumPrecise([-Infinity, -Infinity]) !== -Infinity) {
  throw "expected Math.sumPrecise to preserve negative infinity";
}
