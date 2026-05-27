// Derived from: test/built-ins/Array/isArray/15.4.3.2-0-1.js
if (typeof Array.isArray !== "function") { throw; }
if (!Array.isArray([])) { throw; }
if (Array.isArray({})) { throw; }
if (Array.isArray("abc")) { throw; }
