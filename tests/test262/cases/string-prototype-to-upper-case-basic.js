// Derived from: test/built-ins/String/prototype/toUpperCase/S15.5.4.18_A1_T1.js
if ("abc XYZ 123".toUpperCase() !== "ABC XYZ 123") {
  throw "expected toUpperCase to uppercase ASCII letters";
}
if (String(true).toUpperCase() !== "TRUE") {
  throw "expected toUpperCase to convert boolean strings";
}
