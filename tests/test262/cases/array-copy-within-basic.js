// Derived from: test/built-ins/Array/prototype/copyWithin/non-negative-target-and-start.js
var array = [1, 2, 3, 4, 5];
var result = array.copyWithin(0, 3);

if (result !== array) {
  throw "expected copyWithin to return the receiver";
}
if (array.length !== 5) {
  throw "expected length to stay unchanged";
}
if (array[0] !== 4 || array[1] !== 5 || array[2] !== 3 || array[3] !== 4 || array[4] !== 5) {
  throw "expected elements to be copied";
}
if (Array.prototype.copyWithin.length !== 2) {
  throw "expected Array.prototype.copyWithin.length to be 2";
}
if (Array.prototype.copyWithin.propertyIsEnumerable("length")) {
  throw "expected Array.prototype.copyWithin.length to be non-enumerable";
}
