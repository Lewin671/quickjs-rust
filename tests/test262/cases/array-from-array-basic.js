// Derived from: test/built-ins/Array/from/from-array.js
var source = [0, "foo", undefined, Infinity];
var result = Array.from(source);
if (result.length !== 4 || result[0] !== 0 || result[1] !== "foo" || result[2] !== undefined || result[3] !== Infinity) {
  throw "Array.from should copy array values";
}
if (result === source || !Array.isArray(result)) {
  throw "Array.from should return a new array";
}
