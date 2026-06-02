// Derived from: test/built-ins/Object/keys/15.2.3.14-1-5.js
var caught = false;
try {
  Object.keys(undefined);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "Object.keys(undefined) should throw TypeError";
}
