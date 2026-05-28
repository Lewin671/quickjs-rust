// Derived from: test/built-ins/Array/prototype/reverse/S15.4.4.8_A1_T1.js
var array = [1, 2, 3];
var result = array.reverse();

if (result !== array) {
  throw "expected reverse to return the receiver";
}
if (array.length !== 3) {
  throw "expected length to stay unchanged";
}
if (array[0] !== 3 || array[1] !== 2 || array[2] !== 1) {
  throw "expected elements to be reversed";
}
if (Array.prototype.reverse.length !== 0) {
  throw "expected Array.prototype.reverse.length to be 0";
}
if (Array.prototype.reverse.propertyIsEnumerable("length")) {
  throw "expected Array.prototype.reverse.length to be non-enumerable";
}
