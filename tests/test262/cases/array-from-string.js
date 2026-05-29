// Derived from: test/built-ins/Array/from/from-string.js
var result = Array.from("Test");
if (result.length !== 4 || result[0] !== "T" || result[1] !== "e" || result[2] !== "s" || result[3] !== "t") {
  throw "Array.from should create an array from a string";
}
