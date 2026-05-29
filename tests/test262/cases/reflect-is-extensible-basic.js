// Derived from: test/built-ins/Reflect/isExtensible/return-boolean.js
var object = {};
if (Reflect.isExtensible(object) !== true) {
  throw "expected fresh object to be extensible";
}
Object.preventExtensions(object);
if (Reflect.isExtensible(object) !== false) {
  throw "expected prevented object to be non-extensible";
}
