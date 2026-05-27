// Derived from: test/built-ins/String/prototype/concat/S15.5.4.6_A1_T1.js
if ("a".concat("b", 3, true) !== "ab3true") {
  throw "expected concat to append string conversions";
}
if ("".concat() !== "") {
  throw "expected concat with no arguments to return the receiver";
}
