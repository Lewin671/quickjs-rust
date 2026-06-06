// Derived from: test/built-ins/Object/assign/assignment-to-readonly-property-of-target-must-throw-a-typeerror-exception.js
var caught = false;
try {
  Object.assign("a", [1]);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Object.assign to throw when setting a readonly target property";
}
