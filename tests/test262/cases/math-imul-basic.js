// Derived from: test/built-ins/Math/imul/results.js
if (Math.imul(2, 4) !== 8) {
  throw "expected Math.imul(2, 4) to return 8";
}
if (Math.imul(-1, 8) !== -8) {
  throw "expected Math.imul(-1, 8) to return -8";
}
if (Math.imul(4294967295, 5) !== -5) {
  throw "expected Math.imul(4294967295, 5) to return -5";
}
if (Math.imul(1.9, 7) !== 7) {
  throw "expected Math.imul to convert operands with ToUint32";
}
