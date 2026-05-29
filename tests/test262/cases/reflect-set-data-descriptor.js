// Derived from: test/built-ins/Reflect/set/set-value-on-data-descriptor.js
var object = { value: 43 };
if (Reflect.set(object, "value", 42) !== true) {
  throw "expected Reflect.set to return true when updating a writable property";
}
if (object.value !== 42) {
  throw "expected Reflect.set to update a writable property";
}

var target = { value: 43 };
var receiver = { value: 44 };
if (Reflect.set(target, "value", 42, receiver) !== true) {
  throw "expected Reflect.set to return true when updating a receiver";
}
if (target.value !== 43 || receiver.value !== 42) {
  throw "expected Reflect.set to update receiver rather than target";
}
