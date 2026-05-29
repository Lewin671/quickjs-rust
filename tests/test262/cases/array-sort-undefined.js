// Derived from: test/built-ins/Array/prototype/sort/S15.4.4.11_A1.4_T1.js
var array = ["b", undefined, "a"];
array.sort();
if (array.join("|") !== "a|b|") {
  throw "Array.prototype.sort should move undefined elements after defined values";
}
if (array[2] !== undefined) {
  throw "Array.prototype.sort should preserve undefined elements at the end";
}
