// Derived from: test/built-ins/Number/isSafeInteger/not-a-constructor.js
var caught = false;
try {
  new Number.isSafeInteger();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Number.isSafeInteger not to be a constructor";
}
