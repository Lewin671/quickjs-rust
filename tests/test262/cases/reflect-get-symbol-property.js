// Derived from: test/built-ins/Reflect/get/return-value-from-symbol-key.js
var object = {};
var symbol = Symbol("1");
object[symbol] = 42;

if (Reflect.get(object, symbol) !== 42) {
  throw "expected Reflect.get to read a symbol property key";
}
