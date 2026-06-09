// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T4.js

var executed = /a[a-z]{2,4}?/.exec({ toString: function() { return "abcdefghi"; } });

if (!(executed instanceof Array)) {
  throw "expected exec result array";
}
if (executed.length !== 1) {
  throw "expected one match element";
}
if (executed[0] !== "abc") {
  throw "expected shortest counted quantifier match";
}
if (executed.index !== 0) {
  throw "expected match index";
}
if (executed.input !== "abcdefghi") {
  throw "expected coerced input";
}
