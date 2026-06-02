// Derived from: test/built-ins/Object/entries/tamper-with-global-object.js
function fakeObject() {
  throw "called";
}
fakeObject.entries = Object.entries;

var global = Function("return this;")();
global.Object = fakeObject;

if (Object !== fakeObject) {
  throw "global Object was not replaced";
}
if (Object.entries(1).length !== 0) {
  throw "expected number primitive to have zero entries";
}
