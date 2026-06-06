// Derived from: test/language/rest-parameters/rest-parameters-apply.js
function af(...a) {
  return a.length;
}

if (af.apply(null, []) !== 0) {
  throw "expected no applied rest arguments";
}
if (af.apply(null, [1]) !== 1) {
  throw "expected one applied rest argument";
}
if (af.apply(null, [1, 2]) !== 2) {
  throw "expected two applied rest arguments";
}
if (af.apply(null, [1, , 2]) !== 3) {
  throw "expected sparse applied rest arguments";
}
