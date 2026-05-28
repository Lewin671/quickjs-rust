// Derived from: test/built-ins/Reflect/has/has.js
var object = { own: 1 };
var child = Object.create({ inherited: 2 });
if (Reflect.has(object, "own") !== true) {
  throw "expected own property";
}
if (Reflect.has(child, "inherited") !== true) {
  throw "expected inherited property";
}
if (Reflect.has(object, "missing") !== false) {
  throw "expected missing property to be false";
}
