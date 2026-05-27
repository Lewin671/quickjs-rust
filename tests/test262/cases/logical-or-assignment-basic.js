// Derived from: test/language/expressions/logical-assignment/lgcl-or-assignment-operator.js
var value = undefined;
if ((value ||= 1) !== 1) {
  throw;
}

value = false;
if ((value ||= 2) !== 2) {
  throw;
}

value = 3;
if ((value ||= 4) !== 3) {
  throw;
}
