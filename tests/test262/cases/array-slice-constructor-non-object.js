// Derived from: test/built-ins/Array/prototype/slice/create-ctor-non-object.js
var a = [];
a.constructor = 1;
var caught = false;
try {
  a.slice();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) { throw; }
