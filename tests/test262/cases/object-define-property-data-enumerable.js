// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-4.js
var object = {};
Object.defineProperty(object, "foo", { value: 1, enumerable: true });
if (Object.keys(object)[0] !== "foo") { throw; }
if (!object.propertyIsEnumerable("foo")) { throw; }
