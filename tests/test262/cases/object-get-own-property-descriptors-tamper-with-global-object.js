// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/tamper-with-global-object.js
function fakeObject() {
  throw "called";
}
fakeObject.getOwnPropertyDescriptors = Object.getOwnPropertyDescriptors;
fakeObject.keys = Object.keys;

var global = this;
global.Object = fakeObject;

if (Object !== fakeObject) {
  throw "global Object was not replaced";
}
if (Object.keys(Object.getOwnPropertyDescriptors("a")).length !== 2) {
  throw "expected string primitive to have two descriptors";
}
