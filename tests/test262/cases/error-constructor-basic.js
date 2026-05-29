// Derived from: test/built-ins/Error/is-a-constructor.js
var value = new Error("boom");
if (typeof Error !== "function") { throw; }
if (Error.length !== 1) { throw; }
if (value instanceof Error !== true) { throw; }
if (value.constructor !== Error) { throw; }
