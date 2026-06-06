// Derived from: test/language/rest-parameters/rest-parameters-call.js
function af(...a) {
  return a.length;
}

if (af.call(null) !== 0) {
  throw "expected no called rest arguments";
}
if (af.call(null, 1) !== 1) {
  throw "expected one called rest argument";
}
if (af.call(null, 1, 2) !== 2) {
  throw "expected two called rest arguments";
}
