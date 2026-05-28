// Derived from: test/built-ins/Reflect/deleteProperty/delete-properties.js
var object = { value: 1 };
if (Reflect.deleteProperty(object, "value") !== true) {
  throw "expected Reflect.deleteProperty to return true";
}
if (Reflect.has(object, "value") !== false) {
  throw "expected property to be removed";
}
if (Reflect.deleteProperty(object, "missing") !== true) {
  throw "expected missing property deletion to return true";
}
