// Derived from: test/built-ins/Reflect/getOwnPropertyDescriptor/getOwnPropertyDescriptor.js
var descriptor = Reflect.getOwnPropertyDescriptor({ value: 1 }, "value");
if (descriptor.value !== 1) {
  throw "expected descriptor value";
}
if (descriptor.enumerable !== true) {
  throw "expected enumerable descriptor";
}
if (Reflect.getOwnPropertyDescriptor({}, "missing") !== undefined) {
  throw "expected missing descriptor to be undefined";
}
