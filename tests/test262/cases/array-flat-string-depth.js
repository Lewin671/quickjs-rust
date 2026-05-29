// Derived from: test/built-ins/Array/prototype/flat/non-numeric-depth-should-not-throw.js
var array = [1, [2]];
if (array.flat("1").join() !== "1,2") {
  throw "Array.prototype.flat should coerce numeric string depth";
}
if (array.flat("x").join("|") !== "1|2") {
  throw "Array.prototype.flat should coerce non-numeric depth to zero";
}
