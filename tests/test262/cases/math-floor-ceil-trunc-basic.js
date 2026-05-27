// Derived from: test/built-ins/Math/floor/S15.8.2.9_A1.js
if (Math.floor(1.8) !== 1) {
  throw "expected Math.floor(1.8) to return 1";
}
if (Math.ceil(1.2) !== 2) {
  throw "expected Math.ceil(1.2) to return 2";
}
if (Math.trunc(-1.8) !== -1) {
  throw "expected Math.trunc(-1.8) to truncate toward zero";
}
