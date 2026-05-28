// Derived from: test/built-ins/Reflect/defineProperty/defineProperty.js
function fn() {}
if (Reflect.defineProperty(fn, "value", { value: 2, enumerable: true }) !== true) {
  throw "expected Reflect.defineProperty to define function property";
}
if (fn.value !== 2) {
  throw "expected function property value";
}
if (Object.keys(fn).join() !== "value") {
  throw "expected enumerable function property";
}
