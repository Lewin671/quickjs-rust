// Derived from: test/built-ins/String/prototype/toLowerCase/S15.5.4.16_A1_T1.js
if ("ABC xyz 123".toLowerCase() !== "abc xyz 123") {
  throw "expected toLowerCase to lowercase ASCII letters";
}
if (String(true).toLowerCase() !== "true") {
  throw "expected toLowerCase to convert boolean strings";
}
