// Derived from: test/language/expressions/logical-assignment/lgcl-nullish-assignment-operator.js
var value = undefined;
if ((value ??= 1) !== 1) {
  throw;
}

value = null;
if ((value ??= 2) !== 2) {
  throw;
}

value = false;
if ((value ??= 3) !== false) {
  throw;
}
