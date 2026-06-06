// Derived from: test/built-ins/Number/parseInt/not-a-constructor.js
var caught = false;
try {
  new Number.parseInt();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Number.parseInt not to be a constructor";
}
