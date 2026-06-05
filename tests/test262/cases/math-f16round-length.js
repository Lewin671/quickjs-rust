// Derived from: test/built-ins/Math/f16round/length.js
if (Math.f16round.length !== 1) {
  throw "expected Math.f16round.length to be 1";
}
var descriptor = Object.getOwnPropertyDescriptor(Math.f16round, "length");
if (descriptor.enumerable || descriptor.writable || !descriptor.configurable) {
  throw "expected Math.f16round.length descriptor";
}
