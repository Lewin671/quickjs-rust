// Derived from: test/built-ins/Reflect/has/symbol-property.js
var object = {};
var symbol = Symbol("1");
object[symbol] = 42;
var other = Symbol("1");

if (Reflect.has(object, symbol) !== true) {
  throw "expected Reflect.has to find a symbol property key";
}
if (Reflect.has(object, other) !== false) {
  throw "expected Reflect.has to distinguish different symbols";
}
