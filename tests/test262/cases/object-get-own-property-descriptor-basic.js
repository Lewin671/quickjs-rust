// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-1.js
var object = {};
object.foo = 101;
var desc = Object.getOwnPropertyDescriptor(object, "foo");
if (desc.value !== 101) { throw; }
if (desc.enumerable !== true) { throw; }
if (desc.writable !== true) { throw; }
if (desc.configurable !== true) { throw; }
if (desc.hasOwnProperty("get")) { throw; }
if (desc.hasOwnProperty("set")) { throw; }
