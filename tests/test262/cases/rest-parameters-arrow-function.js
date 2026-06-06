// Derived from: test/language/rest-parameters/arrow-function.js
var fn = (a, b, ...c) => c;

if (fn().length !== 0) {
  throw "expected empty rest array";
}
if (fn(1, 2).length !== 0) {
  throw "expected empty rest array after positional arguments";
}
if (fn(1, 2, 3).join() !== "3") {
  throw "expected one rest argument";
}
if (fn(1, 2, 3, 4).join() !== "3,4") {
  throw "expected two rest arguments";
}
if (((...args) => args)(1, 2, 3).join() !== "1,2,3") {
  throw "expected all arguments in rest array";
}
