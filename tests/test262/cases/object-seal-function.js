// Derived from: test/built-ins/Object/seal/object-seal-o-is-a-function-object.js
function fn() {}
Object.seal(fn);
if (Object.isSealed(fn) !== true) throw new Error("function should be sealed");
if (Object.getOwnPropertyDescriptor(fn, "length").configurable !== false) throw new Error("function length should be non-configurable");
