// Derived from: test/language/expressions/this/11.1.1-1.js
if (this === undefined) { throw; }
if (typeof this !== "object") { throw; }
