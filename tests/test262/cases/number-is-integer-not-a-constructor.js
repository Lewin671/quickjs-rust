// Derived from: test/built-ins/Number/isInteger/not-a-constructor.js
var caught = false;
try {
  new Number.isInteger();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Number.isInteger not to be a constructor";
}
