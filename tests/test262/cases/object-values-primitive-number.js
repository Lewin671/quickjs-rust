// Derived from: test/built-ins/Object/values/primitive-numbers.js
var values = Object.values(0);
if (!Array.isArray(values) || values.length !== 0) {
  throw "Object.values should return an empty array for number primitives";
}
