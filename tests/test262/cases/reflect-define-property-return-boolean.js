// Derived from: test/built-ins/Reflect/defineProperty/return-boolean.js
var object = {};
Object.preventExtensions(object);
if (Reflect.defineProperty(object, "value", { value: 1 }) !== false) {
  throw "expected false for non-extensible target";
}
var fixed = {};
Object.defineProperty(fixed, "value", { value: 1 });
if (Reflect.defineProperty(fixed, "value", { configurable: true }) !== false) {
  throw "expected false for incompatible descriptor";
}
