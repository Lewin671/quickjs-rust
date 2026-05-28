// Derived from: test/built-ins/parseInt/S15.1.2.2_A1_T1.js
if (parseInt("15px") !== 15) {
  throw "expected parseInt to stop at first invalid digit";
}
if (parseInt("  -10", 10) !== -10) {
  throw "expected parseInt to trim whitespace and preserve sign";
}
if (parseInt("0x10") !== 16) {
  throw "expected parseInt to infer hexadecimal prefix";
}
if (parseInt("10", 2) !== 2) {
  throw "expected parseInt to honor radix";
}
if (parseInt("z", 36) !== 35) {
  throw "expected parseInt to support radix 36";
}
