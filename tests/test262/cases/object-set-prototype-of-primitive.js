// Derived from: test/built-ins/Object/setPrototypeOf/o-not-obj.js
if (Object.setPrototypeOf(true, null) !== true) throw new Error("boolean should be returned");
if (Object.setPrototypeOf(3, null) !== 3) throw new Error("number should be returned");
if (Object.setPrototypeOf("string", null) !== "string") throw new Error("string should be returned");
