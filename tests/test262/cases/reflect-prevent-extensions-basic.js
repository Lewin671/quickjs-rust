// Derived from: test/built-ins/Reflect/preventExtensions/always-return-true-from-ordinary-object.js
var object = {};
if (Reflect.preventExtensions(object) !== true) {
  throw "expected Reflect.preventExtensions to return true";
}
if (Object.isExtensible(object) !== false) {
  throw "expected object to become non-extensible";
}
if (Reflect.preventExtensions(object) !== true) {
  throw "expected repeated Reflect.preventExtensions to return true";
}
