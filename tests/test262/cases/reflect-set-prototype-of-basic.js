// Derived from: test/built-ins/Reflect/setPrototypeOf/setPrototypeOf.js
var proto = { marker: 7 };
var object = {};
if (Reflect.setPrototypeOf(object, proto) !== true) {
  throw "expected Reflect.setPrototypeOf to return true";
}
if (Reflect.getPrototypeOf(object) !== proto) {
  throw "expected updated prototype";
}
if (object.marker !== 7) {
  throw "expected inherited property";
}
