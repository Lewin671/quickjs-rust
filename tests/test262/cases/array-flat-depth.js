// Derived from: test/built-ins/Array/prototype/flat/positive-infinity.js
var array = [1, [2, [3, [4]]]];
if (array.flat(2).join() !== "1,2,3,4") {
  throw "Array.prototype.flat should honor finite depth";
}
if (array.flat(Infinity).join() !== "1,2,3,4") {
  throw "Array.prototype.flat should honor Infinity depth";
}
