// Derived from: test/built-ins/String/prototype/substring/S15.5.4.15_A3_T2.js
if ("1,2,3,4,5".substring(9, -Infinity) !== "1,2,3,4,5") {
  throw "expected substring to swap clamped start and end";
}
