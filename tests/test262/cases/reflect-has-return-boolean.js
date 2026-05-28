// Derived from: test/built-ins/Reflect/has/return-boolean.js
var result = Reflect.has({ value: 1 }, "value");
if (result !== true) {
  throw "expected true boolean";
}
if (Reflect.has({}, "value") !== false) {
  throw "expected false boolean";
}
