// Derived from: test/built-ins/Number/parseFloat/not-a-constructor.js
var caught = false;
try {
  new Number.parseFloat();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Number.parseFloat not to be a constructor";
}
