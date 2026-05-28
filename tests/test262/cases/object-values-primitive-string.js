// Derived from: test/built-ins/Object/values/primitive-strings.js
var values = Object.values("abc");
if (values.length !== 3 || values[0] !== "a" || values[1] !== "b" || values[2] !== "c") {
  throw "Object.values should return string index values";
}
