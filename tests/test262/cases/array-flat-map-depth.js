// Derived from: test/built-ins/Array/prototype/flatMap/depth-always-one.js
var result = [1, 2].flatMap(function(value) {
  return [[value]];
});
if (result.length !== 2 || !Array.isArray(result[0]) || result[0][0] !== 1 || result[1][0] !== 2) {
  throw "Array.prototype.flatMap should flatten exactly one level";
}
