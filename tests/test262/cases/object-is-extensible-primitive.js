// Derived from: test/built-ins/Object/isExtensible/15.2.3.13-1-1.js
if (Object.isExtensible(1) !== false) throw new Error("number should not be extensible");
if (Object.isExtensible("x") !== false) throw new Error("string should not be extensible");
if (Object.isExtensible(null) !== false) throw new Error("null should not be extensible");
if (Object.isExtensible(undefined) !== false) throw new Error("undefined should not be extensible");
