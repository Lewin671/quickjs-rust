// Derived from: test/built-ins/String/prototype/match/S15.5.4.10_A2_T4.js
var matches = "123456abcde7890".match(/\d{2}/g);
var expected = ["12", "34", "56", "78", "90"];
if (matches.length !== expected.length) {
  throw "expected digit escape global match count";
}
for (var index = 0; index < expected.length; index++) {
  if (matches[index] !== expected[index]) {
    throw "expected digit escape global match value";
  }
}
