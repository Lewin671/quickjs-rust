// Derived from: test/built-ins/Number/isFinite/not-a-constructor.js
var caught = false;
try {
  new Number.isFinite();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Number.isFinite not to be a constructor";
}
