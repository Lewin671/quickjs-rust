// Derived from: test/built-ins/Math/sumPrecise/sum-is-minus-zero.js
if (1 / Math.sumPrecise([]) !== -Infinity) {
  throw "expected Math.sumPrecise empty input to return -0";
}
if (1 / Math.sumPrecise([-0]) !== -Infinity) {
  throw "expected Math.sumPrecise all -0 input to return -0";
}
if (1 / Math.sumPrecise([-0, -0]) !== -Infinity) {
  throw "expected Math.sumPrecise repeated -0 input to return -0";
}
if (1 / Math.sumPrecise([-0, 0]) !== Infinity) {
  throw "expected Math.sumPrecise mixed zero input to return +0";
}
