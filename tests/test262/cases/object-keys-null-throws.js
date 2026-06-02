// Derived from: test/built-ins/Object/keys/15.2.3.14-1-4.js
var caught = false;
try {
  Object.keys(null);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "Object.keys(null) should throw TypeError";
}
