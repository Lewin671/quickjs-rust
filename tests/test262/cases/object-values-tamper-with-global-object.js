// Derived from: test/built-ins/Object/values/tamper-with-global-object.js
function fakeObject() {
  throw "called";
}
fakeObject.values = Object.values;

var global = Function("return this;")();
global.Object = fakeObject;

if (Object !== fakeObject) {
  throw "global Object was not replaced";
}
if (Object.values(1).length !== 0) {
  throw "expected number primitive to have zero values";
}
