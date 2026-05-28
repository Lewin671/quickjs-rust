// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/normal-object.js
var result = Object.getOwnPropertyDescriptors({});
if (Object.getPrototypeOf(result) !== Object.prototype) {
  throw "Object.getOwnPropertyDescriptors should return an ordinary object";
}
