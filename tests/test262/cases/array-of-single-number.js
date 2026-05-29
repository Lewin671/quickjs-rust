// Derived from: test/built-ins/Array/of/creates-a-new-array-from-arguments.js
var values = Array.of(3);
if (values.length !== 1 || values[0] !== 3) {
  throw "Array.of should not treat a single number as an array length";
}
