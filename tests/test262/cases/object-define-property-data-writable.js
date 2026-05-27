// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-2.js
var object = {};
Object.defineProperty(object, "foo", { value: 1 });
object.foo = 2;
if (object.foo !== 1) { throw; }
Object.defineProperty(object, "bar", { value: 3, writable: true });
object.bar = 4;
if (object.bar !== 4) { throw; }
