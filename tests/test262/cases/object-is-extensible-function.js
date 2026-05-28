// Derived from: test/built-ins/Object/isExtensible/15.2.3.13-2-3.js
function fn() {}
if (Object.isExtensible(fn) !== true) throw new Error("function should be extensible");
Object.preventExtensions(fn);
if (Object.isExtensible(fn) !== false) throw new Error("function should not be extensible");
