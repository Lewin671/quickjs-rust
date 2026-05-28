// Derived from: test/built-ins/Reflect/defineProperty/defineProperty.js
var object = {};
if (Reflect.defineProperty(object, "value", {
  value: 1,
  enumerable: true,
  writable: true,
  configurable: true
}) !== true) {
  throw "expected Reflect.defineProperty to return true";
}
if (object.value !== 1) {
  throw "expected property value to be defined";
}
if (Object.keys(object).join() !== "value") {
  throw "expected enumerable property";
}
