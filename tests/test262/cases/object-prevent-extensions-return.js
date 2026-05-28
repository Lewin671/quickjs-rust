// Derived from: test/built-ins/Object/preventExtensions/15.2.3.10-2.js
let object = {};
if (Object.preventExtensions(object) !== object) throw new Error("should return target");
if (Object.preventExtensions(1) !== 1) throw new Error("should return primitive target");
