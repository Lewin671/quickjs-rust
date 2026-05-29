// Derived from: test/built-ins/Array/prototype/toReversed/length-tolength.js
var result = Array.prototype.toReversed.call({ length: "3", 0: "a", 2: "c" });
if (result.length !== 3 || result[0] !== "c" || result[1] !== undefined || result[2] !== "a") {
  throw "Array.prototype.toReversed should read array-like values in reverse order";
}
