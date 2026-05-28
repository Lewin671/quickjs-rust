// Derived from: test/built-ins/Object/is/not-same-value-x-y-type.js
if (Object.is(1, "1")) {
  throw "Object.is should reject values with different types";
}
if (Object.is(false, true)) {
  throw "Object.is should reject different booleans";
}
if (Object.is(null, undefined)) {
  throw "Object.is should reject null and undefined";
}
if (Object.is("same", "different")) {
  throw "Object.is should reject different strings";
}
