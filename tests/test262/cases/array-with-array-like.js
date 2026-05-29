// Derived from: test/built-ins/Array/prototype/with/length-tolength.js
var result = Array.prototype.with.call({ length: "3", 0: "a", 2: "c" }, 1, "b");
if (result.join("|") !== "a|b|c") {
  throw "Array.prototype.with should copy and replace array-like values";
}
