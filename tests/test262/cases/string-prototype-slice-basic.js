// Derived from: test/built-ins/String/prototype/slice/S15.5.4.13_A1_T1.js
if ("abcdef".slice(1, 4) !== "bcd") {
  throw "expected slice to return selected range";
}
if ("abcdef".slice(-3) !== "def") {
  throw "expected slice to resolve negative start";
}
if ("abcdef".slice(4, 1) !== "") {
  throw "expected slice with end before start to return empty string";
}
