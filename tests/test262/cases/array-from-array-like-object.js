// Derived from: test/built-ins/Array/from/source-object-length.js
var result = Array.from({ length: 3, 0: "a", 2: "c" });
if (result.length !== 3 || result[0] !== "a" || result[1] !== undefined || result[2] !== "c") {
  throw "Array.from should copy indexed values from array-like objects";
}
