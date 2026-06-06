// Derived from: test/built-ins/Number/isNaN/not-a-constructor.js
var caught = false;
try {
  new Number.isNaN();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Number.isNaN not to be a constructor";
}
