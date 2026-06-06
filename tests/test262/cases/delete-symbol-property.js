// Derived from: test/language/expressions/delete/S8.12.7_A3.js
var symbol = Symbol("1");
var object = {};
object[symbol] = 42;

if (delete object[symbol] !== true) {
  throw "expected delete operator to delete a symbol property";
}
if (object.hasOwnProperty(symbol) !== false) {
  throw "expected deleted symbol property to be absent";
}
if (object[symbol] !== undefined) {
  throw "expected deleted symbol property value to be undefined";
}
