// Derived from: test/built-ins/Array/of/creates-a-new-array-from-arguments.js
var values = Array.of();
if (values.length !== 0 || !Array.isArray(values)) {
  throw "Array.of with no arguments should create an empty array";
}
