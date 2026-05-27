// Derived from: test/built-ins/Math/round/length.js
if (Math.round.length !== 1) {
  throw "expected Math.round.length to be 1";
}

if (Math.round.propertyIsEnumerable("length")) {
  throw "expected Math.round.length to be non-enumerable";
}
