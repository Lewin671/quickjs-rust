// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-2.js
var desc = Object.getOwnPropertyDescriptor({}, "foo");
if (desc !== undefined) { throw; }
