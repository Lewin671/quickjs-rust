// Derived from: test/built-ins/Reflect/construct/return-with-newtarget-argument.js
var capturedPrototype;

function C() {
  capturedPrototype = Object.getPrototypeOf(this);
}

var result = Reflect.construct(C, [], Array);

if (Object.getPrototypeOf(result) !== Array.prototype) {
  throw "Reflect.construct should use newTarget prototype for the result";
}
if (capturedPrototype !== Array.prototype) {
  throw "Reflect.construct should expose the newTarget prototype inside target";
}
