// Derived from: test/language/expressions/in/S8.12.6_A1.js
var obj = { fooProp: "fooooooo" };
if (!("fooProp" in obj)) { throw; }
