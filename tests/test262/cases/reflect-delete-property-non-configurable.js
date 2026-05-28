// Derived from: test/built-ins/Reflect/deleteProperty/return-boolean.js
var object = {};
Object.defineProperty(object, "fixed", { value: 1 });
if (Reflect.deleteProperty(object, "fixed") !== false) {
  throw "expected non-configurable property deletion to return false";
}
