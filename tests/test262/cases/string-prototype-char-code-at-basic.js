// Derived from: test/built-ins/String/prototype/charCodeAt/S15.5.4.5_A1_T1.js
if ("abc".charCodeAt(0) !== 97) {
  throw "expected charCodeAt(0) to return first code unit";
}
if ("abc".charCodeAt(1) !== 98) {
  throw "expected charCodeAt(1) to return second code unit";
}
if ("abc".charCodeAt() !== 97) {
  throw "expected omitted position to select index zero";
}
