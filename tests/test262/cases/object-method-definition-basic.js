// Derived from: test/language/expressions/object/method-definition/name-params.js
var value1 = {};
var value2 = {};
var value3 = {};
var arg1;
var arg2;
var arg3;
var object = {
  method(a, b, c) {
    arg1 = a;
    arg2 = b;
    arg3 = c;
  }
};

object.method(value1, value2, value3);

if (arg1 !== value1) {
  throw;
}
if (arg2 !== value2) {
  throw;
}
if (arg3 !== value3) {
  throw;
}
