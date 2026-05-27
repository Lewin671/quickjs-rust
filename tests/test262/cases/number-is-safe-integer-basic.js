// Derived from: test/built-ins/Number/isSafeInteger/safe-integers.js
if (!Number.isSafeInteger(9007199254740991)) {
  throw "expected max safe integer to return true";
}
if (!Number.isSafeInteger(-9007199254740991)) {
  throw "expected min safe integer to return true";
}
if (Number.isSafeInteger(9007199254740992)) {
  throw "expected unsafe integer to return false";
}
if (Number.isSafeInteger(1.5)) {
  throw "expected non-integer to return false";
}
