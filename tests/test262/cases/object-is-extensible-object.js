// Derived from: test/built-ins/Object/isExtensible/15.2.3.13-2-1.js
let object = {};
if (Object.isExtensible(object) !== true) throw new Error("object should be extensible");
Object.preventExtensions(object);
if (Object.isExtensible(object) !== false) throw new Error("object should not be extensible");
