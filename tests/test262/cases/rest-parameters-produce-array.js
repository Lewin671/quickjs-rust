// Derived from: test/language/rest-parameters/rest-parameters-produce-an-array.js
function af(...a) {
  if (a.constructor !== Array) {
    throw "expected rest parameter constructor to be Array";
  }
  if (!Array.isArray(a)) {
    throw "expected rest parameter to be an array";
  }
}

af(1);
