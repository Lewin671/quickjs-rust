// Derived from: test/language/expressions/in/S8.12.6_A3.js
var obj = {};
obj.hole = undefined;
if (obj.hole !== undefined) { throw; }
if (obj.notexist !== undefined) { throw; }
if (!("hole" in obj)) { throw; }
if ("notexist" in obj) { throw; }
