// Derived from: test/built-ins/Object/keys/15.2.3.14-5-13.js
var array = [1, , 3, , 5];

Object.defineProperty(array, 5, {
  value: 7,
  enumerable: false,
  configurable: true
});

Object.defineProperty(array, 10000, {
  value: "ElementWithLargeIndex",
  enumerable: true,
  configurable: true
});

if (Object.keys(array).join("|") !== "0|2|4|10000") {
  throw "Object.keys should preserve sparse array index descriptor enumerability";
}
