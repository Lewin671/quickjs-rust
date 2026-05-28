// Derived from: test/built-ins/Reflect/setPrototypeOf/return-false-if-target-is-not-extensible.js
var object = {};
Object.preventExtensions(object);
if (Reflect.setPrototypeOf(object, null) !== false) {
  throw "expected false for non-extensible target";
}
