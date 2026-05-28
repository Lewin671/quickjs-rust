// Derived from: test/built-ins/Object/isSealed/15.2.3.11-1.js
if (Object.isSealed(0) !== true) throw new Error("number should be sealed");
if (Object.isSealed(null) !== true) throw new Error("null should be sealed");
if (Object.isSealed(undefined) !== true) throw new Error("undefined should be sealed");
