// Derived from: test/built-ins/Array/prototype/flat/null-undefined-elements.js
var actual = [1, [2, 3], 4].flat();
if (actual.join() !== "1,2,3,4") {
  throw "Array.prototype.flat should flatten one level by default";
}
