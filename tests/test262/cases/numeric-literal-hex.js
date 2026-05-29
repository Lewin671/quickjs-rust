// Derived from: test/language/literals/numeric/S7.8.3_A5.1_T1.js
if (0x0 !== 0) {
  throw "expected 0x0 to be 0";
}
if (0xF !== 15) {
  throw "expected 0xF to be 15";
}
if (0X100 !== 256) {
  throw "expected 0X100 to be 256";
}
