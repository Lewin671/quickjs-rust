// Derived from: test/built-ins/Array/prototype/unshift/read-only-property.js
var caught = false;
try {
  Array.prototype.unshift.call({ get 0() {} }, 0);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected unshift to throw when target property is read-only";
}
