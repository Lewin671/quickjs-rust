// Derived from: test/built-ins/Object/getPrototypeOf/15.2.3.2-2-1.js
var proto = {};
var object = Object.create(proto);
if (Object.getPrototypeOf(object) !== proto) { throw; }
