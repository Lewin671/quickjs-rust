// Derived from: test/built-ins/Object/seal/object-seal-returned-object-is-not-extensible.js
let object = {};
if (Object.seal(object) !== object) throw new Error("should return target");
if (Object.isExtensible(object) !== false) throw new Error("sealed object should not be extensible");
