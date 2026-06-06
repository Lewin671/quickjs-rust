// Derived from: test/built-ins/Math/sumPrecise/throws-on-non-number.js
function expectTypeError(value) {
  var caught = false;
  try {
    Math.sumPrecise(value);
  } catch (error) {
    caught = error instanceof TypeError;
  }
  if (!caught) {
    throw "expected Math.sumPrecise to reject non-number elements";
  }
}

expectTypeError([{}]);
expectTypeError(["1"]);
expectTypeError([1, "2"]);
