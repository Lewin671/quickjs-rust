// Derived from: test/built-ins/Object/isExtensible/15.2.3.13-2-15.js
if (Object.isExtensible([]) !== true) throw new Error("array should be extensible");
let array = [];
Object.preventExtensions(array);
if (Object.isExtensible(array) !== false) throw new Error("array should not be extensible");
