// Derived from: test/built-ins/Math/sumPrecise/not-a-constructor.js
var caught = false;
try {
  new Math.sumPrecise();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Math.sumPrecise not to be a constructor";
}
