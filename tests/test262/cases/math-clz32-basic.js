// Derived from: test/built-ins/Math/clz32/Math.clz32.js
if (Math.clz32(0) !== 32) {
  throw "expected Math.clz32(0) to return 32";
}
if (Math.clz32(-0) !== 32) {
  throw "expected Math.clz32(-0) to return 32";
}
if (Math.clz32(1) !== 31) {
  throw "expected Math.clz32(1) to return 31";
}
if (Math.clz32(4294967295) !== 0) {
  throw "expected Math.clz32(4294967295) to return 0";
}
