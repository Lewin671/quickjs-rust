// Derived from: test/built-ins/Object/is/same-value-x-y-object.js
var object = {};
if (!Object.is(object, object)) {
  throw "Object.is should compare objects by identity";
}
if (Object.is({}, {})) {
  throw "Object.is should reject different object identities";
}
