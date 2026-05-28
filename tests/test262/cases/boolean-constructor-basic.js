// Derived from: test/built-ins/Boolean/S15.6.1.1_A1_T1.js
if (typeof Boolean() !== "boolean") {
  throw "expected Boolean() to return a boolean primitive";
}
if (Boolean() !== false) {
  throw "expected Boolean() with no arguments to return false";
}
if (Boolean(1) !== true) {
  throw "expected Boolean(1) to return true";
}
if (Boolean(0) !== false) {
  throw "expected Boolean(0) to return false";
}
if (Boolean("") !== false) {
  throw "expected Boolean empty string to return false";
}
if (Boolean("x") !== true) {
  throw "expected Boolean non-empty string to return true";
}
if (typeof new Boolean(true) !== "object") {
  throw "expected new Boolean(true) to return an object";
}
