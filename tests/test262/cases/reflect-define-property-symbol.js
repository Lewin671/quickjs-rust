// Derived from: test/built-ins/Reflect/defineProperty/define-symbol-properties.js
var symbol = Symbol("1");
var object = {};

if (Reflect.defineProperty(object, symbol, { value: 42 }) !== true) {
  throw "expected Reflect.defineProperty to define a symbol property";
}
if (object[symbol] !== 42) {
  throw "expected defined symbol property value";
}
