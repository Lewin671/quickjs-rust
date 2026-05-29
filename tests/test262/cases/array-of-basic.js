// Derived from: test/built-ins/Array/of/creates-a-new-array-from-arguments.js
var values = Array.of("Mike", "Rick", "Leo");
if (values.length !== 3 || values[0] !== "Mike" || values[1] !== "Rick" || values[2] !== "Leo") {
  throw "Array.of should create an array from its arguments";
}

var mixed = Array.of(undefined, false, null, undefined);
if (mixed.length !== 4 || mixed[0] !== undefined || mixed[1] !== false || mixed[2] !== null || mixed[3] !== undefined) {
  throw "Array.of should preserve primitive, null, and undefined values";
}
