// Derived from: test/built-ins/Reflect/set/creates-a-data-descriptor.js
var object = {};
if (Reflect.set(object, "value", 42) !== true) {
  throw "expected Reflect.set to return true for a new data property";
}
if (object.value !== 42) {
  throw "expected Reflect.set to create a data property";
}

var target = {};
var receiver = {};
if (Reflect.set(target, "value", 43, receiver) !== true) {
  throw "expected Reflect.set to return true with a receiver";
}
if (target.value !== undefined || receiver.value !== 43) {
  throw "expected Reflect.set to write through the receiver";
}
