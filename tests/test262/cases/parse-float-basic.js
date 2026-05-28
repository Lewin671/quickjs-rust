// Derived from: test/built-ins/parseFloat/S15.1.2.3_A1_T1.js
if (parseFloat("3.5px") !== 3.5) {
  throw "expected parseFloat to stop at first invalid character";
}
if (parseFloat("  -1.25e2x") !== -125) {
  throw "expected parseFloat to parse signed exponent form";
}
if (parseFloat("Infinity") !== Infinity) {
  throw "expected parseFloat to parse Infinity";
}
if (parseFloat("-Infinity") !== -Infinity) {
  throw "expected parseFloat to parse negative Infinity";
}
