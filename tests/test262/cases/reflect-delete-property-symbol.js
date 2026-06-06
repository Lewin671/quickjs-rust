// Derived from: test/built-ins/Reflect/deleteProperty/delete-symbol-properties.js
var symbol = Symbol("1");
var object = {};
object[symbol] = 42;

if (Reflect.deleteProperty(object, symbol) !== true) {
  throw "expected Reflect.deleteProperty to delete a symbol property";
}
if (object.hasOwnProperty(symbol) !== false) {
  throw "expected deleted symbol property to be absent";
}
if (object[symbol] !== undefined) {
  throw "expected deleted symbol property value to be undefined";
}
