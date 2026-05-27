// Derived from: test/built-ins/String/prototype/charAt/S15.5.4.4_A1_T1.js
if ("abc".charAt(0) !== "a") {
  throw "expected charAt(0) to return first character";
}
if ("abc".charAt(1) !== "b") {
  throw "expected charAt(1) to return second character";
}
if ("abc".charAt(9) !== "") {
  throw "expected out-of-range charAt to return empty string";
}
