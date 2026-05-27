// Derived from: test/language/expressions/typeof/undefined.js
if (typeof undefined !== "undefined") { throw; }
if (typeof true !== "boolean") { throw; }
if (typeof 1 !== "number") { throw; }
if (typeof "x" !== "string") { throw; }
if (typeof null !== "object") { throw; }
if (typeof {} !== "object") { throw; }
