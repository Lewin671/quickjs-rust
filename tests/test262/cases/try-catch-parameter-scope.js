// Derived from: test/language/statements/try/12.14-8.js
try {
  throw "x";
} catch (e) {
}

if (typeof e !== "undefined") { throw; }
