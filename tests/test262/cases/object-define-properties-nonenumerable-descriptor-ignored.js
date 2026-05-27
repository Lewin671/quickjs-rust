// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-3-1.js
var object = {};
var descriptors = {};
Object.defineProperty(descriptors, "hidden", { value: { value: 1 } });
Object.defineProperties(object, descriptors);
if (object.hasOwnProperty("hidden")) { throw; }
