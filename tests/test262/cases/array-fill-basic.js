// Derived from: test/built-ins/Array/prototype/fill/fill-values-custom-start-and-end.js
var array = [1, 2, 3, 4];
var result = array.fill(0, -3, -1);

if (result !== array) {
  throw "expected fill to return the receiver";
}
if (array.length !== 4) {
  throw "expected length to stay unchanged";
}
if (array[0] !== 1 || array[1] !== 0 || array[2] !== 0 || array[3] !== 4) {
  throw "expected selected elements to be filled";
}
if (Array.prototype.fill.length !== 1) {
  throw "expected Array.prototype.fill.length to be 1";
}
if (Array.prototype.fill.propertyIsEnumerable("length")) {
  throw "expected Array.prototype.fill.length to be non-enumerable";
}
