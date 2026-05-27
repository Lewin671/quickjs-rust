// Derived from: test/language/expressions/logical-assignment/lgcl-and-assignment-operator-unresolved-rhs.js
var value = 0;
if ((value &&= unresolved) !== 0) {
  throw;
}

value = 1;
if ((value ||= unresolved) !== 1) {
  throw;
}

value = false;
if ((value ??= unresolved) !== false) {
  throw;
}
