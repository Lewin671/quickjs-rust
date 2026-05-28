// Derived from: test/built-ins/Boolean/prototype/toString/S15.6.4.2_A1_T1.js
if (Boolean.prototype.toString() !== "false") {
  throw "expected Boolean.prototype.toString() to return false";
}
if (true.toString() !== "true") {
  throw "expected true.toString() to return true";
}
if ((new Boolean(true)).toString() !== "true") {
  throw "expected Boolean object toString to use wrapped value";
}
if ((new Boolean(0)).toString() !== "false") {
  throw "expected Boolean object toString to coerce constructor argument";
}
