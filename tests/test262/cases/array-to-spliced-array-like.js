// Derived from: test/built-ins/Array/prototype/toSpliced/length-tolength.js
var result = Array.prototype.toSpliced.call({ length: "3", 0: "a", 2: "c" }, 1, 1, "b");
if (result.join("|") !== "a|b|c") {
  throw "Array.prototype.toSpliced should copy and splice array-like values";
}
