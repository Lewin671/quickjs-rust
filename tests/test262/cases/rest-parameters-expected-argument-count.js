// Derived from: test/language/rest-parameters/expected-argument-count.js
function af(...a) {}
function bf(a, ...b) {}

if (af.length !== 0) {
  throw "expected rest-only function length";
}
if (bf.length !== 1) {
  throw "expected positional count before rest";
}
