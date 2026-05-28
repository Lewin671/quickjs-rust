// Derived from: test/built-ins/Object/isSealed/15.2.3.11-4-1.js
let object = { value: 1 };
if (Object.isSealed(object) !== false) throw new Error("ordinary object should start unsealed");
Object.seal(object);
if (Object.isSealed(object) !== true) throw new Error("object should be sealed");
