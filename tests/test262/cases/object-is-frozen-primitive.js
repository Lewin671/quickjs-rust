// Derived from: test/built-ins/Object/isFrozen/15.2.3.12-1.js
if (Object.isFrozen(0) !== true) throw new Error("number should be frozen");
if (Object.isFrozen(null) !== true) throw new Error("null should be frozen");
if (Object.isFrozen(undefined) !== true) throw new Error("undefined should be frozen");
