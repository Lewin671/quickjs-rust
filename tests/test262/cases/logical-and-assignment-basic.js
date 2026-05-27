// Derived from: test/language/expressions/logical-assignment/lgcl-and-assignment-operator.js
var value = undefined;
if ((value &&= 1) !== undefined) {
  throw;
}

value = false;
if ((value &&= 1) !== false) {
  throw;
}

value = 2;
if ((value &&= 1) !== 1) {
  throw;
}
