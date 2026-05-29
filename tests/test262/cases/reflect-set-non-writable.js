// Derived from: test/built-ins/Reflect/set/return-false-if-receiver-is-not-writable.js
var object = {};
Object.defineProperty(object, "value", {
  value: 42,
  writable: false
});
if (Reflect.set(object, "value", 43) !== false) {
  throw "expected Reflect.set to return false for a non-writable data property";
}
if (object.value !== 42) {
  throw "expected Reflect.set to preserve non-writable data property value";
}
