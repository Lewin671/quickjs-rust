// Derived from: test/built-ins/Reflect/getPrototypeOf/getPrototypeOf.js
if (typeof Reflect.getPrototypeOf !== "function") {
  throw "expected Reflect.getPrototypeOf to be a function";
}
if (Reflect.getPrototypeOf({}) !== Object.prototype) {
  throw "expected ordinary object prototype";
}
if (Reflect.getPrototypeOf([]) !== Array.prototype) {
  throw "expected array prototype";
}
if (Reflect.getPrototypeOf(Object.create(null)) !== null) {
  throw "expected null prototype";
}
