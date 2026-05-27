// Derived from: test/built-ins/String/prototype/indexOf/S15.5.4.7_A1_T1.js
if ("abcabc".indexOf("bc") !== 1) {
  throw "expected indexOf to find substring";
}
if ("abcabc".indexOf("bc", 3) !== 4) {
  throw "expected indexOf to honor start position";
}
if ("abcabc".indexOf("z") !== -1) {
  throw "expected indexOf to return -1 when missing";
}
