// Derived from: test/built-ins/Reflect/has/symbol-property.js
var symbol = Symbol("1");
var other = Symbol("1");
var object = {};
object[symbol] = 42;

if ((symbol in object) !== true) {
  throw "expected in operator to find a symbol property key";
}
if ((other in object) !== false) {
  throw "expected in operator to distinguish different symbols";
}
