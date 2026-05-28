// Derived from: test/built-ins/Reflect/deleteProperty/delete-properties.js
function fn() {}
fn.value = 2;
if (Reflect.deleteProperty(fn, "value") !== true) {
  throw "expected function property deletion to return true";
}
if (Reflect.has(fn, "value") !== false) {
  throw "expected function property to be removed";
}
