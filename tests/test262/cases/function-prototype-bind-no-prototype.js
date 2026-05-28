// Derived from: test/built-ins/Function/prototype/bind/BoundFunction_restricted-properties.js
function target() {}
var bound = target.bind(null);
if (Object.hasOwn(bound, "prototype")) {
  throw "bound functions should not have an own prototype property";
}
